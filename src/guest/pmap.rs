use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use crate::debug::PageDebug;
use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::page_table::{
    translate_guest_address, PageTable, PageTableEntry, PhysPageNum, PTEFlags,
    VirtPageNum,
};
use crate::constants::layout::{GUEST_KERNEL_VIRT_START, PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT};

use super::{GuestMemory, Vcpu};
use super::shadow_stats::ShadowPageTableUpdate;

// PR #26 (`feature/shadow-page-table-asid`) encodes stable shadow-root ASIDs
// in the architectural RV64 Sv39 field while reserving ASID 0 for the Host.
const SATP_ASID_SHIFT: usize = 44;
const MAX_SV39_ASID: usize = (1 << 16) - 1;

/// 页表(影子页表类型)
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PageTableRoot {
    /// Guest Physical Address
    GPA,
    /// Guest Virtual Address
    GVA,
    /// User Virtual Address
    UVA
}

/// PR #25 (`feature/cache-shadow-page-table-state`) couples each shadow root
/// with the guest PTE generation for which its full synchronization is valid.
struct CachedShadowPageTable<P: PageTable + PageDebug> {
    page_table: P,
    synchronized_generation: usize,
    /// PR #26 (`feature/shadow-page-table-asid`) gives each root an independent
    /// hardware TLB namespace and tracks whether that namespace needs a fence.
    asid: usize,
    tlb_dirty: bool,
}

#[derive(Copy, Clone)]
struct GuestPageTablePageState {
    /// PR #27 (`feature/track-valid-pte-count`) keys metadata by the
    /// guest-physical page containing PTEs, so shared pages have one live count.
    vpn: VirtPageNum,
    valid_pte_count: usize,
}

/// One valid Guest leaf found above Sv39's 4 KiB level.
/// PR #58 (`feature/sv39-superpage-shadowing`) expands it because a VM's Host
/// RAM slot is not guaranteed to preserve the Guest's superpage alignment.
#[derive(Copy, Clone)]
struct GuestSuperpageLeaf {
    base_vpn: usize,
    base_gpa: usize,
    page_count: usize,
    flags: PTEFlags,
}

pub struct ShadowPageTables<P: PageTable + PageDebug> {
    /// all shadow page tables (satp, spt)
    spts: UnsafeCell<BTreeMap<usize, CachedShadowPageTable<P>>>,
    /// guest kernel installed shadow page table
    pub page_tables: [Option<usize>; 3],
    /// kernel guest page table token
    pub guest_satp: Option<usize>,
    /// PR #25 (`feature/cache-shadow-page-table-state`) increments this
    /// generation on trapped guest PTE writes so cached roots detect stale state.
    pte_generation: usize,
    /// PR #26 (`feature/shadow-page-table-asid`) reserves ASID 0 for Hypocaust
    /// and assigns stable, nonzero identifiers to guest shadow roots.
    next_asid: usize,
    /// PR #27 (`feature/track-valid-pte-count`) records V=1 entries while full
    /// walks inspect each page, avoiding a later 512-entry invalidation scan.
    valid_pte_counts: BTreeMap<VirtPageNum, usize>,
}

impl<P> ShadowPageTables<P> where P: PageDebug + PageTable {
    pub const fn new() -> Self {
        Self {
            spts: UnsafeCell::new(BTreeMap::new()),
            page_tables: [None; 3],
            guest_satp: None,
            pte_generation: 0,
            next_asid: 1,
            valid_pte_counts: BTreeMap::new(),
        }
    }

    fn spts(&self) -> &mut BTreeMap<usize, CachedShadowPageTable<P>> {
        unsafe{ &mut *self.spts.get() }
    }

    pub fn push(&mut self, satp: usize, spt: P) -> usize {
        assert!(self.next_asid <= MAX_SV39_ASID, "shadow ASID space exhausted");
        let asid = self.next_asid;
        self.next_asid += 1;
        let shadow_satp = spt.token() | (asid << SATP_ASID_SHIFT);
        self.spts.get_mut().insert(satp, CachedShadowPageTable {
            page_table: spt,
            synchronized_generation: self.pte_generation,
            asid,
            tlb_dirty: false,
        });
        shadow_satp
    }


    pub fn shadow_page_table(&self, satp: usize) -> Option<&mut P> {
        let inner = self.spts();
        inner.get_mut(&satp).map(|cached| &mut cached.page_table)
    }

    pub fn requires_resynchronization(&self, satp: usize) -> Option<bool> {
        let inner = self.spts();
        inner.get(&satp).map(|cached| {
            cached.synchronized_generation != self.pte_generation
        })
    }

    pub fn mark_synchronized(&self, satp: usize) {
        let generation = self.pte_generation;
        self.spts().get_mut(&satp).unwrap().synchronized_generation = generation;
    }

    pub fn record_pte_write(&mut self) {
        self.pte_generation = self.pte_generation.wrapping_add(1);
        self.mark_all_tlb_dirty();
    }

    pub fn shadow_token(&self, satp: usize) -> Option<usize> {
        self.spts().get(&satp).map(|cached| {
            cached.page_table.token() | (cached.asid << SATP_ASID_SHIFT)
        })
    }

    pub fn mark_all_tlb_dirty(&self) {
        self.spts().values_mut().for_each(|cached| {
            cached.tlb_dirty = true;
        });
    }

    /// PR #48 (`fix-bug/guest-exception-forwarding`) identifies pages that
    /// Hypocaust intentionally write-protected for incremental PTE tracking.
    pub fn tracks_page_table_page(&self, gpa: usize) -> bool {
        self.valid_pte_counts
            .contains_key(&VirtPageNum::from(gpa >> 12))
    }

    pub fn take_tlb_flush(&self, satp: usize) -> Option<usize> {
        let cached = self.spts().get_mut(&satp).unwrap();
        if cached.tlb_dirty {
            cached.tlb_dirty = false;
            Some(cached.asid)
        }else{
            None
        }
    }

    fn record_page_table_pages(&mut self, pages: &[GuestPageTablePageState]) {
        pages.iter().for_each(|page| {
            self.valid_pte_counts.insert(page.vpn, page.valid_pte_count);
        });
    }

    /// PR #27 (`feature/track-valid-pte-count`) updates the page's V=1
    /// population in O(1). A page first discovered through a trapped write is
    /// counted once from its updated contents, then follows the incremental path.
    fn update_valid_pte_count<F: FnOnce() -> usize>(
        &mut self,
        page_vpn: VirtPageNum,
        old_pte: PageTableEntry,
        new_pte: PageTableEntry,
        fallback_count: F,
    ) -> (usize, bool) {
        if let Some(count) = self.valid_pte_counts.get_mut(&page_vpn) {
            match (old_pte.is_valid(), new_pte.is_valid()) {
                (false, true) => *count += 1,
                (true, false) => {
                    assert!(*count > 0, "valid PTE count underflow");
                    *count -= 1;
                }
                _ => {}
            }
            (*count, false)
        } else {
            let fallback_count = fallback_count();
            self.valid_pte_counts.insert(page_vpn, fallback_count);
            (fallback_count, true)
        }
    }

    pub fn guest_page_table(&self) -> Option<&mut P> {
        let inner = self.spts();
        if let Some(guest_satp) = self.guest_satp {
            inner.get_mut(&guest_satp).map(|cached| &mut cached.page_table)
        }else{
            None
        }
    }

    pub fn install_root(&mut self, spt_token: usize, mode: PageTableRoot) {
        match mode {
            PageTableRoot::GPA => self.page_tables[0] = Some(spt_token),
            PageTableRoot::GVA => self.page_tables[1] = Some(spt_token),
            PageTableRoot::UVA => self.page_tables[2] = Some(spt_token)
        }
    }

}

/// PR #65 (`fix-bug/linux-first-guest-page-table`) treats the first Sv39 root
/// as the Guest kernel root even when Linux's temporary mapping does not cover
/// the xv6 identity-map probe address. Later non-matching roots remain UVA.
fn classify_page_table_root(
    maps_identity_probe: bool,
    guest_kernel_root_exists: bool,
) -> PageTableRoot {
    if maps_identity_probe || !guest_kernel_root_exists {
        PageTableRoot::GVA
    } else {
        PageTableRoot::UVA
    }
}

pub fn page_table_mode<P: PageTable>(
    page_table: P,
    guest_memory: &GuestMemory,
    guest_kernel_root_exists: bool,
) -> PageTableRoot {
    let maps_identity_probe = page_table
        .translate_guest(VirtPageNum::from(GUEST_KERNEL_VIRT_START >> 12), guest_memory)
        .is_some();
    classify_page_table_root(maps_identity_probe, guest_kernel_root_exists)
}

fn checked_gpa_to_hpa(guest_memory: &GuestMemory, gpa: usize, len: usize) -> usize {
    guest_memory
        .translate_range(gpa, len)
        .unwrap_or_else(|| panic!(
            "Guest page-table range [{:#x}, {:#x}) is outside VM RAM",
            gpa,
            gpa.saturating_add(len),
        ))
}

fn checked_gpa_to_shadow_hpa(guest_memory: &GuestMemory, gpa: usize, len: usize) -> usize {
    guest_memory
        .translate_shadow_range(gpa, len)
        .unwrap_or_else(|| panic!(
            "Guest page-table range [{:#x}, {:#x}) is outside the VM shadow slot",
            gpa,
            gpa.saturating_add(len),
        ))
}

/// PR #35 (`fix-bug/non-ram-shadow-leaves`) keeps non-RAM leaves invalid.
/// PR #36 (`feature/vm-guest-memory`) applies that policy through the owning
/// VM's checked memory capability instead of global address bounds.
fn shadow_leaf_pte(
    guest_memory: &GuestMemory,
    guest_pte: PageTableEntry,
) -> PageTableEntry {
    let guest_page = guest_pte.ppn().0 << 12;
    match guest_memory.translate_range(guest_page, PAGE_SIZE) {
        Some(host_page) => PageTableEntry::new(
            PhysPageNum::from(host_page >> 12),
            guest_pte.flags() | PTEFlags::U,
        ),
        None => PageTableEntry::empty(),
    }
}

fn is_leaf(pte: PageTableEntry) -> bool {
    pte.readable() || pte.executable()
}

fn superpage_leaf(pte: PageTableEntry, walk: usize, base_vpn: usize) -> Option<GuestSuperpageLeaf> {
    if !pte.is_valid() || !is_leaf(pte) || (pte.writable() && !pte.readable()) || walk >= 2 {
        return None;
    }
    let page_count = 1usize << ((2 - walk) * 9);
    let base_gpa = pte.ppn().0 << 12;
    if base_gpa & (page_count * PAGE_SIZE - 1) != 0 {
        return None;
    }
    Some(GuestSuperpageLeaf {
        base_vpn,
        base_gpa,
        page_count,
        flags: pte.flags(),
    })
}

/// PR #60 (`fix-bug/superpage-expansion-path`) removes an upper-level Guest
/// leaf before PageTable::map builds the Host-only lower levels beneath it.
/// Leaving the Guest PPN installed would make the Host walker interpret Guest
/// RAM as a Host page-table frame while expanding the superpage.
fn prepare_superpage_leaf(
    guest_pte: PageTableEntry,
    host_pte: &mut PageTableEntry,
    walk: usize,
    base_vpn: usize,
) -> Option<GuestSuperpageLeaf> {
    *host_pte = PageTableEntry::empty();
    superpage_leaf(guest_pte, walk, base_vpn)
}

fn install_superpage_leaves<P: PageTable>(
    spt: &mut P,
    guest_memory: &GuestMemory,
    leaves: &[GuestSuperpageLeaf],
) {
    for leaf in leaves {
        for page in 0..leaf.page_count {
            let gpa = leaf.base_gpa + page * PAGE_SIZE;
            let Some(hpa) = guest_memory.translate_range(gpa, PAGE_SIZE) else {
                continue;
            };
            let vpn = VirtPageNum::from(leaf.base_vpn + page);
            let ppn = PhysPageNum::from(hpa >> 12);
            let flags = leaf.flags | PTEFlags::U | PTEFlags::V;
            if let Some(pte) = spt.find_pte(vpn) {
                *pte = PageTableEntry::new(ppn, flags);
            } else {
                spt.map(vpn, ppn, flags);
            }
        }
    }
}

/// Install a Host-private leaf without assuming its slot is empty. PR #67
/// (`fix-bug/idempotent-shadow-leaf-mapping`) makes this operation idempotent
/// because Linux UVA roots can share the lower-level shadow path where the
/// kernel root already installed the trampoline and trap-context leaves.
fn install_hypervisor_leaf<P: PageTable>(
    spt: &mut P,
    vpn: VirtPageNum,
    ppn: PhysPageNum,
    flags: PTEFlags,
) {
    if let Some(pte) = spt.find_pte(vpn) {
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    } else {
        spt.map(vpn, ppn, flags);
    }
}

/// PR #58 validates Sv39 leaf classification, size, and alignment without
/// allocating the hundreds of 4 KiB leaves used by a real 1 GiB mapping.
pub(crate) fn superpage_self_test() {
    // PR #65 verifies Linux's first root becomes the protection root without
    // weakening the established identity-map classification for xv6-rust.
    assert!(matches!(
        classify_page_table_root(false, false),
        PageTableRoot::GVA,
    ));
    assert!(matches!(
        classify_page_table_root(true, true),
        PageTableRoot::GVA,
    ));
    assert!(matches!(
        classify_page_table_root(false, true),
        PageTableRoot::UVA,
    ));

    let aligned = PageTableEntry::new(
        PhysPageNum::from(GUEST_KERNEL_VIRT_START >> 12),
        PTEFlags::V | PTEFlags::R | PTEFlags::X,
    );
    assert_eq!(
        superpage_leaf(aligned, 0, 0).map(|leaf| leaf.page_count),
        Some(1 << 18),
    );
    assert_eq!(
        superpage_leaf(aligned, 1, 0).map(|leaf| leaf.page_count),
        Some(1 << 9),
    );
    let misaligned = PageTableEntry::new(
        PhysPageNum::from((GUEST_KERNEL_VIRT_START >> 12) + 1),
        PTEFlags::V | PTEFlags::R,
    );
    assert!(superpage_leaf(misaligned, 1, 0).is_none());

    let mut shadow_leaf = aligned;
    assert!(prepare_superpage_leaf(aligned, &mut shadow_leaf, 1, 0).is_some());
    assert!(
        !shadow_leaf.is_valid(),
        "superpage expansion must detach the Guest leaf before allocating Host levels",
    );

    // PR #67 reproduces the shared-shadow-path case: a UVA root can inherit
    // the Host trampoline leaf that was already installed for the kernel root.
    let mut special_pages = crate::page_table::PageTableSv39::new();
    let special_vpn = VirtPageNum::from(TRAMPOLINE >> 12);
    let special_ppn = PhysPageNum::from(0x12345);
    install_hypervisor_leaf(
        &mut special_pages,
        special_vpn,
        special_ppn,
        PTEFlags::R | PTEFlags::X,
    );
    install_hypervisor_leaf(
        &mut special_pages,
        special_vpn,
        special_ppn,
        PTEFlags::R | PTEFlags::X,
    );
    assert_eq!(
        special_pages.translate(special_vpn).map(|pte| pte.ppn()),
        Some(special_ppn),
        "Host-private shadow leaves must be idempotent",
    );
}



fn update_pte_readonly<P: PageTable>(vpn: VirtPageNum, spt: &mut P) -> bool {
    if let Some(pte) = spt.find_pte(vpn) {
        if pte.writable() | pte.executable() {
            *pte = PageTableEntry::new(pte.ppn(), PTEFlags::R | PTEFlags::U | PTEFlags::V);
        }
        true
    }else{
        false
    }
}

/// Protect every virtual alias of every Guest page-table page in one shadow
/// root. PR #66 (`feature/shadow-page-table-alias-tracking`) replaces the old
/// identity-address-only assumption with a reverse walk over shadow leaves, so
/// Linux direct-map writes fault into the incremental PTE synchronizer.
fn protect_page_table_aliases<P: PageTable>(
    shadow_page_table: &mut P,
    guest_memory: &GuestMemory,
    page_table_pages: &[GuestPageTablePageState],
) -> usize {
    // PR #66 uses a compact sorted vector rather than a tree set here. This
    // walk runs on the bounded per-vCPU Host stack, and binary search keeps
    // lookups logarithmic without the deeper B-tree insertion call chain.
    let mut protected_host_ppns: Vec<usize> = page_table_pages
        .iter()
        .filter_map(|page| {
            guest_memory
                .translate_range(page.vpn.0 << 12, PAGE_SIZE)
                .map(|hpa| hpa >> 12)
        })
        .collect();
    protected_host_ppns.sort_unstable();
    protected_host_ppns.dedup();
    let mut queue = VecDeque::new();
    let mut protected_aliases = 0;
    queue.push_back((shadow_page_table.root_ppn(), 0usize));

    while let Some((page_table_ppn, level)) = queue.pop_front() {
        for pte in page_table_ppn.get_pte_array().iter_mut() {
            if !pte.is_valid() {
                continue;
            }
            if is_leaf(*pte) {
                // PR #58 expands valid Guest superpages into 4 KiB Host
                // leaves, allowing one aliased page to be protected precisely.
                if level == 2
                    && protected_host_ppns.binary_search(&pte.ppn().0).is_ok()
                {
                    *pte = PageTableEntry::new(
                        pte.ppn(),
                        PTEFlags::R | PTEFlags::U | PTEFlags::V,
                    );
                    protected_aliases += 1;
                }
            } else if level < 2 {
                queue.push_back((pte.ppn(), level + 1));
            }
        }
    }
    protected_aliases
}

fn clear_page_table<P: PageTable>(spt: &mut P, va: usize, valid_pte_count: usize) {
    if valid_pte_count == 0 {
        // htracking!("Drop the page table guest ppn -> {:#x}", guest_ppn.0);
        // 将影子页表设置为可读可写
        if let Some(spt_pte) = spt.find_pte(VirtPageNum::from(va >> 12)) {
            *spt_pte = PageTableEntry::new(spt_pte.ppn(), PTEFlags::R | PTEFlags::W | PTEFlags::U | PTEFlags::V);
        }
    }
}

/// 收集所有页表的虚拟页号
fn collect_page_table_pages<P: PageTable>(
    guest_memory: &GuestMemory,
    satp: usize,
) -> Vec<GuestPageTablePageState> {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;

    // 遍历所有页表项
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    // 非叶子所在的虚拟页号
    let mut page_table_pages = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back(vpn);

    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let guest_page_table_vpn = queue.pop_front().unwrap();
            // 获得 guest pte 的物理页号
            let guest_page_table_ppn = PhysPageNum::from(
                checked_gpa_to_hpa(guest_memory, guest_page_table_vpn.0 << 12, PAGE_SIZE) >> 12
            );
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            let mut valid_pte_count = 0;
            for guest_pte in guest_ptes.iter(){
                if guest_pte.is_valid() {
                    valid_pte_count += 1;
                }
                if guest_pte.is_valid() && walk < 2 && !is_leaf(*guest_pte) {
                    // 非叶子页表项
                    buffer.push(VirtPageNum::from(guest_pte.ppn().0));
                }else if guest_pte.is_valid() && walk == 2 {
                }
            }
            page_table_pages.push(GuestPageTablePageState {
                vpn: guest_page_table_vpn,
                valid_pte_count,
            });
        }
        while !buffer.is_empty() {
            queue.push_back(buffer.pop().unwrap());
        }
    }
    page_table_pages
    
}

fn synchronize_page_table<P: PageTable>(
    guest_memory: &GuestMemory,
    satp: usize,
    shadow_page_table: &mut P,
) -> Vec<GuestPageTablePageState> {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;

    // 遍历所有页表项
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    let mut superpages = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back((vpn, 0usize));
    let mut page_table_pages = Vec::new();

    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let (guest_page_table_vpn, virtual_prefix) = queue.pop_front().unwrap();
            // 收集所有非叶子节点 `vpn`，用于设置为只读
            let host_page_table_ppn = PhysPageNum::from(
                checked_gpa_to_shadow_hpa(guest_memory, guest_page_table_vpn.0 << 12, PAGE_SIZE) >> 12
            );
            // 获得 guest pte 的物理页号
            let guest_page_table_ppn = PhysPageNum::from(
                checked_gpa_to_hpa(guest_memory, guest_page_table_vpn.0 << 12, PAGE_SIZE) >> 12
            );
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            // 获得 host pte 页表项内容
            let host_ptes = host_page_table_ppn.get_pte_array();
            let mut valid_pte_count = 0;
            for (index, guest_pte) in guest_ptes.iter().enumerate() {
                let entry_vpn = virtual_prefix | (index << ((2 - walk) * 9));
                if guest_pte.is_valid() {
                    valid_pte_count += 1;
                }
                if guest_pte.is_valid() && walk < 2 && !is_leaf(*guest_pte) {
                    // 非叶子页表项
                    buffer.push((VirtPageNum::from(guest_pte.ppn().0), entry_vpn));
                    // 构造 host pte
                    let host_pte = PageTableEntry::new(
                        PhysPageNum::from(checked_gpa_to_shadow_hpa(
                            guest_memory, guest_pte.ppn().0 << 12, PAGE_SIZE
                        ) >> 12),
                        guest_pte.flags(),
                    );
                    host_ptes[index] = host_pte;
                } else if guest_pte.is_valid() && walk < 2 && is_leaf(*guest_pte) {
                    // PR #60 makes the mirrored path safe before deferred
                    // expansion. Misaligned Guest superpages remain invalid.
                    if let Some(leaf) = prepare_superpage_leaf(
                        *guest_pte,
                        &mut host_ptes[index],
                        walk,
                        entry_vpn,
                    ) {
                        superpages.push(leaf);
                    }
                } else if guest_pte.is_valid() && walk == 2 {
                    host_ptes[index] = shadow_leaf_pte(guest_memory, *guest_pte);
                }
            }
            page_table_pages.push(GuestPageTablePageState {
                vpn: guest_page_table_vpn,
                valid_pte_count,
            });
        }
        while !buffer.is_empty() {
            queue.push_back(buffer.pop().unwrap());
        }
    }
    // PR #59 (`fix-bug/superpage-resync-frame-ownership`) installs expanded
    // leaves through the cached PageTable owner. Any lower-level frames
    // allocated by PageTable::map therefore live as long as the cached root;
    // a temporary from_ppn wrapper would free them on return and leave dangling
    // PTEs in the Guest's active shadow tree.
    install_superpage_leaves(shadow_page_table, guest_memory, &superpages);
    // PR #66 reapplies alias protection after a full cached-root refresh,
    // because synchronization may have restored writable Guest leaf flags.
    protect_page_table_aliases(shadow_page_table, guest_memory, &page_table_pages);
    page_table_pages
}

/// 用于初始化影子页表同步所有页表项(仅在最开始时使用)
fn initialize_shadow_page_table<P: PageTable>(
    guest_memory: &GuestMemory,
    satp: usize,
    mode: PageTableRoot,
    guest_spt: Option<&mut P>,
) -> Option<(P, Vec<GuestPageTablePageState>)> {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;
    let host_root_pa = checked_gpa_to_shadow_hpa(guest_memory, guest_root_pa, PAGE_SIZE);
    // 获取 `guest SPT`
    let mut empty_spt = P::from_token(0);
    let guest_spt = match mode {
        PageTableRoot::GVA => { &mut empty_spt },
        PageTableRoot::UVA => if let Some(spt) = guest_spt { spt } else { panic!() }
        _ => unreachable!() 
    };
    // 遍历所有页表项
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    let mut superpages = Vec::new();
    // 非叶子所在的虚拟页号
    let mut page_table_pages = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back((vpn, 0usize));
    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let (guest_page_table_vpn, virtual_prefix) = queue.pop_front().unwrap();
            let host_page_table_ppn = PhysPageNum::from(
                checked_gpa_to_shadow_hpa(guest_memory, guest_page_table_vpn.0 << 12, PAGE_SIZE) >> 12
            );
            // 获得 guest pte 的物理页号
            let guest_page_table_ppn = PhysPageNum::from(
                checked_gpa_to_hpa(guest_memory, guest_page_table_vpn.0 << 12, PAGE_SIZE) >> 12
            );
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            // 获得 host pte 页表项内容
            let host_ptes = host_page_table_ppn.get_pte_array();
            let mut valid_pte_count = 0;
            for (index, guest_pte) in guest_ptes.iter().enumerate() {
                let entry_vpn = virtual_prefix | (index << ((2 - walk) * 9));
                if guest_pte.is_valid() {
                    valid_pte_count += 1;
                }
                if guest_pte.is_valid() && walk < 2 && !is_leaf(*guest_pte) {
                    // 非叶子页表项
                    buffer.push((VirtPageNum::from(guest_pte.ppn().0), entry_vpn));
                    // 构造 host pte
                    let host_pte = PageTableEntry::new(
                        PhysPageNum::from(checked_gpa_to_shadow_hpa(
                            guest_memory, guest_pte.ppn().0 << 12, PAGE_SIZE
                        ) >> 12),
                        guest_pte.flags(),
                    );
                    host_ptes[index] = host_pte;
                } else if guest_pte.is_valid() && walk < 2 && is_leaf(*guest_pte) {
                    // PR #58 defers expansion until the mirrored hierarchy is
                    // complete; PR #60 first detaches the Guest leaf so
                    // PageTable::map allocates a genuine Host-owned path.
                    if let Some(leaf) = prepare_superpage_leaf(
                        *guest_pte,
                        &mut host_ptes[index],
                        walk,
                        entry_vpn,
                    ) {
                        superpages.push(leaf);
                    }
                } else if guest_pte.is_valid() && walk == 2 {
                    host_ptes[index] = shadow_leaf_pte(guest_memory, *guest_pte);
                }
            }
            page_table_pages.push(GuestPageTablePageState {
                vpn: guest_page_table_vpn,
                valid_pte_count,
            });
        }
        while !buffer.is_empty() {
            queue.push_back(buffer.pop().unwrap());
        }
    }
    let mut host_shadow_page_table = PageTable::from_ppn(PhysPageNum::from(host_root_pa >> 12));
    install_superpage_leaves(&mut host_shadow_page_table, guest_memory, &superpages);
    // PR #66 protects Linux's high direct-map aliases as well as xv6-rust's
    // identity aliases before the new shadow root can execute.
    protect_page_table_aliases(
        &mut host_shadow_page_table,
        guest_memory,
        &page_table_pages,
    );
    page_table_pages.iter().for_each(|page| {
        match mode {
            PageTableRoot::GVA => {
                update_pte_readonly(page.vpn, &mut host_shadow_page_table);
            },
            PageTableRoot::UVA => {
                update_pte_readonly(page.vpn, guest_spt);
            },
            _ => unreachable!()
        }
    });
    Some((host_shadow_page_table, page_table_pages))
}




impl<P> Vcpu<P> where P: PageDebug + PageTable {
    /// GPA -> HPA
    pub fn translate_guest_paddr(&self, paddr: usize) -> Option<usize> {
        let offset = paddr & 0xfff;
        let vpn: VirtPageNum = VirtPageNum::from(paddr >> 12);
        let pte = self.translate_guest_ppte(vpn);
        if let Some(pte) = pte {
            return Some((pte.ppn(). 0 << 12) + offset)
        }
        None
    }

    /// GVA -> HPA
    pub fn translate_guest_vaddr(&self, vaddr: usize) -> Option<usize> {
        let offset = vaddr & 0xfff;
        let vpn = VirtPageNum::from(vaddr >> 12);
        let pte = self.translate_guest_vpte(vpn);
        if let Some(pte) = pte {
            return Some((pte.ppn(). 0 << 12) + offset)
        }
        None
    }

    /// Translate a Guest virtual address into the Guest-physical address space.
    /// PR #68 (`fix-bug/plic-mmio-shadow-fault`) keeps this separate from
    /// `translate_guest_vaddr`, whose result is a Host address and therefore
    /// cannot represent emulated MMIO outside the VM's RAM slot.
    pub fn translate_guest_vaddr_to_gpa(&self, vaddr: usize) -> Option<usize> {
        if self.shadow() == PageTableRoot::GPA {
            return Some(vaddr);
        }
        let guest_root = (self.shadow_state.csrs.satp & 0xfff_ffff_ffff) << 12;
        translate_guest_address::<P>(&self.guest_memory, guest_root, vaddr)
            .map(|translation| translation.guest_pa)
    }

    pub fn translate_guest_ppte(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.memory_set.translate(vpn)
    }

    pub fn translate_guest_vpte(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        if let Some(spt) = self.shadow_state.shadow_page_tables.shadow_page_table(self.shadow_state.csrs.satp) {
            // 由于 GHA 与 GPA 是同等映射的，因此翻译成的物理地址可以直接当虚拟地址用
            spt.translate(vpn)
        }else{
            // hwarning!("translate guest va from GPA mode?");
            self.translate_guest_ppte(vpn)
        }
    }

    pub fn translate_valid_guest_vaddr(&self, vaddr: usize) -> Option<usize> {
        let offset = vaddr & 0xfff;
        let vpn = VirtPageNum::from(vaddr >> 12);
        let pte = self.translate_guest_vpte(vpn);
        if let Some(pte) = pte {
            if !pte.is_valid(){ return None }
            return Some((pte.ppn(). 0 << 12) + offset)
        }
        None
    }

    /// 根据 satp 构建影子页表
    /// 需要将 GVA -> HPA
    pub fn make_shadow_page_table(&mut self, satp: usize) {
        // PR #24 (`feature/shadow-paging-profile`) measures the complete shadow
        // update, including cache lookup, walks, and special-page mappings.
        let start_cycles = read_cycle();
        let mut full_walks = 0;
        let mut walked_page_table_pages = 0;
        let update;
        let guest_memory = &self.guest_memory;
        // PR #25 (`feature/cache-shadow-page-table-state`) still classifies the
        // live root because a guest may reuse a root PPN; only freshness is cached.
        let root_gpa = (satp & 0xfff_ffff_ffff) << 12;
        let root_hppn = PhysPageNum::from(
            checked_gpa_to_hpa(guest_memory, root_gpa, PAGE_SIZE) >> 12
        );
        let gpt = P::from_ppn(root_hppn);
        let guest_kernel_root_exists = self
            .shadow_state
            .shadow_page_tables
            .guest_satp
            .is_some();
        let mode = page_table_mode(gpt, guest_memory, guest_kernel_root_exists);
        let requires_resynchronization = self.shadow_state.shadow_page_tables
            .requires_resynchronization(satp);
        if requires_resynchronization.is_none() {
            update = ShadowPageTableUpdate::New;
            // 如果影子页表中没有发现，新建影子页表
            let mut spt;
            // 根据页表是否可读内核地址空间判断是 `GVA` 还是 `UVA`
            match mode {
                PageTableRoot::GVA => {
                    let initialized = initialize_shadow_page_table::<P>(guest_memory, satp, mode, None).unwrap();
                    spt = initialized.0;
                    walked_page_table_pages += initialized.1.len();
                    full_walks += 1;
                    self.shadow_state.shadow_page_tables.record_page_table_pages(&initialized.1);
                    self.shadow_state.shadow_page_tables.guest_satp = Some(satp);

                    // PR #46 (`fix-bug/invalid-leaf-translation`) exposes the
                    // intentionally trapped VirtIO page as an absent mapping.
                    assert!(spt.translate(VirtPageNum::from(0x10001)).is_none());
                }
                PageTableRoot::UVA => {
                    // 同步 guest spt,即将用户页表设置为只读
                    // PR #65 makes the first root GVA, so every later UVA root
                    // has a concrete protection root instead of unwrapping None.
                    let guest_spt = self
                        .shadow_state
                        .shadow_page_tables
                        .guest_page_table()
                        .expect("secondary Guest root requires a kernel shadow root");
                    let initialized = initialize_shadow_page_table::<P>(guest_memory, satp, mode, Some(guest_spt)).unwrap();
                    spt = initialized.0;
                    walked_page_table_pages += initialized.1.len();
                    full_walks += 1;
                    self.shadow_state.shadow_page_tables.record_page_table_pages(&initialized.1);
                    
                }
                _ => unreachable!()
            }

            // 为 `SPT` 映射跳板页
            // 无论是 guest spt 还是 user spt 都要映射跳板页与 Trap Context
            let hypervisor_memory = HYPERVISOR_MEMORY.exclusive_access();
            let trampoline_hppn = hypervisor_memory
                .translate(VirtPageNum::from(TRAMPOLINE >> 12))
                .unwrap()
                .ppn();
            // PR #67 permits a new UVA root to retain and refresh a mapping
            // already inherited through a shared kernel shadow path.
            install_hypervisor_leaf(
                &mut spt,
                VirtPageNum::from(TRAMPOLINE >> 12),
                trampoline_hppn,
                PTEFlags::R | PTEFlags::X,
            );

            let trapctx_hvpn =
                VirtPageNum::from(self.translate_guest_paddr(TRAP_CONTEXT).unwrap() >> 12);
            let trapctx_hppn = hypervisor_memory.translate(trapctx_hvpn).unwrap().ppn();
            install_hypervisor_leaf(
                &mut spt,
                VirtPageNum::from(TRAP_CONTEXT >> 12),
                trapctx_hppn,
                PTEFlags::R | PTEFlags::W,
            );

            // hdebug!("Make new SPT(satp -> {:#x}, spt -> {:#x}) ", satp, spt.token());
            let shadow_satp = self.shadow_state.shadow_page_tables.push(satp, spt);
            self.shadow_state.shadow_page_tables.install_root(shadow_satp, mode);
        }else{
            let requires_resynchronization = requires_resynchronization.unwrap();
            match mode {
                PageTableRoot::GVA => {
                    update = ShadowPageTableUpdate::CachedKernel;
                    // os 的内存映射几乎不会改变,因此在切换页表时不需要同步
                    self.shadow_state.conseutive_satp_switch_count += 1;
                    if requires_resynchronization {
                        // PR #25 (`feature/cache-shadow-page-table-state`) only
                        // revisits cached pages after a guest PTE was written.
                        let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
                        let page_table_pages = collect_page_table_pages::<P>(guest_memory, satp);
                        walked_page_table_pages += page_table_pages.len();
                        full_walks += 1;
                        page_table_pages.iter().for_each(|page| {
                            update_pte_readonly(page.vpn, guest_spt);
                        });
                        self.shadow_state.shadow_page_tables.record_page_table_pages(&page_table_pages);
                        self.shadow_state.shadow_page_tables.mark_synchronized(satp);
                    }
                },
                PageTableRoot::UVA => {
                    update = ShadowPageTableUpdate::CachedUser;
                    if requires_resynchronization {
                        let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
                        let page_table_pages = collect_page_table_pages::<P>(guest_memory, satp);
                        walked_page_table_pages += page_table_pages.len();
                        full_walks += 1;
                        page_table_pages.iter().for_each(|page| {
                            update_pte_readonly(page.vpn, guest_spt);
                        });
                        self.shadow_state.shadow_page_tables.record_page_table_pages(&page_table_pages);
                        // 需要更新用户态页表
                        let spt = self
                            .shadow_state
                            .shadow_page_tables
                            .shadow_page_table(satp)
                            .unwrap();
                        let synchronized_pages =
                            synchronize_page_table::<P>(guest_memory, satp, spt);
                        walked_page_table_pages += synchronized_pages.len();
                        full_walks += 1;
                        let hypervisor_memory = HYPERVISOR_MEMORY.exclusive_access();
                        // 为 `SPT` 映射跳板页
                        let trampoline_hppn = hypervisor_memory
                            .translate(VirtPageNum::from(TRAMPOLINE >> 12))
                            .unwrap()
                            .ppn();
                        // PR #67 reapplies Host-private mappings after a full
                        // Guest refresh, whether the shared slot is absent,
                        // invalid, or already contains the desired leaf.
                        install_hypervisor_leaf(
                            spt,
                            VirtPageNum::from(TRAMPOLINE >> 12),
                            trampoline_hppn,
                            PTEFlags::R | PTEFlags::X,
                        );

                        let trapctx_hvpn = VirtPageNum::from(
                            self.translate_guest_paddr(TRAP_CONTEXT).unwrap() >> 12,
                        );
                        let trapctx_hppn =
                            hypervisor_memory.translate(trapctx_hvpn).unwrap().ppn();
                        install_hypervisor_leaf(
                            spt,
                            VirtPageNum::from(TRAP_CONTEXT >> 12),
                            trapctx_hppn,
                            PTEFlags::R | PTEFlags::W,
                        );
                        self.shadow_state
                            .shadow_page_tables
                            .record_page_table_pages(&synchronized_pages);
                        self.shadow_state.shadow_page_tables.mark_synchronized(satp);
                    }
                    let shadow_satp = self.shadow_state.shadow_page_tables.shadow_token(satp).unwrap();
                    self.shadow_state.shadow_page_tables.install_root(shadow_satp, PageTableRoot::UVA);
                },
                _ => unreachable!()
            }
        }
        if full_walks != 0 {
            // PR #26 (`feature/shadow-page-table-asid`) invalidates tagged
            // translations after a full walk may rewrite or protect shadow PTEs.
            self.shadow_state.shadow_page_tables.mark_all_tlb_dirty();
        }
        let elapsed_cycles = read_cycle().wrapping_sub(start_cycles);
        self.shadow_state.shadow_paging_stats.record_satp_update(
            update,
            full_walks,
            walked_page_table_pages,
            elapsed_cycles,
        );
    }



    pub fn synchronize_page_table(
        &mut self,
        va: usize,
        old_pte: PageTableEntry,
        pte: PageTableEntry,
    ) {
        let guest_memory = &self.guest_memory;
        // 获取对应影子页表的地址
        let host_pa = checked_gpa_to_shadow_hpa(
            guest_memory,
            va,
            core::mem::size_of::<PageTableEntry>(),
        );
        let host_ppn = PhysPageNum::from(host_pa >> 12);
        if va % core::mem::size_of::<PageTableEntry>() != 0 {
            panic!("Page Table Entry aligned?");
        }
        let page_vpn = VirtPageNum::from(va >> 12);
        let (valid_pte_count, fallback_scan) = self
            .shadow_state
            .shadow_page_tables
            .update_valid_pte_count(page_vpn, old_pte, pte, || {
                let page_gpa = va & !(PAGE_SIZE - 1);
                let guest_ppn = PhysPageNum::from(
                    checked_gpa_to_hpa(guest_memory, page_gpa, PAGE_SIZE) >> 12
                );
                guest_ppn
                    .get_pte_array()
                    .iter()
                    .filter(|pte| pte.is_valid())
                    .count()
            });
        let mut newly_linked_page_table = None;
        if pte.is_valid() && !(pte.readable() | pte.writable() | pte.executable()) {
            // PR #48 records a newly linked non-leaf page immediately. Its
            // first trapped write can then be distinguished from an ordinary
            // protection fault without relying on Guest-controlled alignment.
            let child_gpa = pte.ppn().0 << 12;
            if let Some(child_hpa) = guest_memory.translate_range(child_gpa, PAGE_SIZE) {
                let child_ppn = PhysPageNum::from(child_hpa >> 12);
                let child_state = GuestPageTablePageState {
                    vpn: VirtPageNum::from(pte.ppn().0),
                    valid_pte_count: child_ppn
                        .get_pte_array()
                        .iter()
                        .filter(|entry| entry.is_valid())
                        .count(),
                };
                self.shadow_state
                    .shadow_page_tables
                    .record_page_table_pages(&[child_state]);
                newly_linked_page_table = Some(child_state);
            }
        }
        // 获得影子页表
        let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
        if !pte.is_valid() {
            // PR #21 (fix-bug/invalid-pte-synchronization): mirror every V=0 encoding,
            // including allocator metadata, and release pages with no valid PTEs.
            unsafe{ core::ptr::write(host_pa as *mut usize, pte.bits as usize) };
            // 消除页表映射，将页表内存修改为可读可写
            clear_page_table(guest_spt, va, valid_pte_count);
        }else {
            // 如果页表项对齐且物理页号不为零表示进行页表映射
            let index = (host_pa & 0xfff) / core::mem::size_of::<PageTableEntry>();
            let pte_array = host_ppn.get_pte_array();
            if pte.is_valid() && (pte.readable() | pte.writable() | pte.executable()) {
                // 叶子节点
                pte_array[index] = shadow_leaf_pte(guest_memory, pte);
                let vpn = VirtPageNum::from(va >> 12);
                if let Some(pte) = guest_spt.translate(vpn) {
                    if pte.writable() | pte.executable() {
                        htracking!("Allocate page table, ppn: {:#x}", vpn.0);
                        update_pte_readonly(vpn, guest_spt);
                    }
                }

            }else if pte.is_valid() && !(pte.readable() | pte.writable() | pte.executable()) {
                // 非叶子节点
                // 获取非叶子节点的偏移
                if let Some(new_hpa) = guest_memory
                    .translate_shadow_range(pte.ppn().0 << 12, PAGE_SIZE)
                {
                    let new_ppn = PhysPageNum::from(new_hpa >> 12);
                    let new_flags = pte.flags();
                    let new_pte = PageTableEntry::new(new_ppn, new_flags);
                    pte_array[index] = new_pte;
                    // 判断当前页面是否设置为只读
                    let vpn = VirtPageNum::from(va >> 12);
                    if let Some(pte) = guest_spt.translate(vpn) {
                        if pte.writable() | pte.executable() {
                            htracking!("Allocate page table, ppn: {:#x}", vpn.0);
                            update_pte_readonly(vpn, guest_spt);
                        }
                    }
                }else{
                    // PR #48 mirrors an out-of-RAM Guest non-leaf as invalid.
                    // A later access becomes a Guest page fault instead of a
                    // Host panic in checked shadow-memory translation.
                    pte_array[index] = PageTableEntry::empty();
                }
            }
        }
        if let Some(child_state) = newly_linked_page_table {
            // PR #66 immediately reverse-protects every alias of a newly
            // linked lower-level table. Otherwise Linux can populate that
            // table through its direct map before the next full root refresh.
            let current_satp = self.shadow_state.csrs.satp;
            if let Some(current_spt) = self
                .shadow_state
                .shadow_page_tables
                .shadow_page_table(current_satp)
            {
                protect_page_table_aliases(
                    current_spt,
                    guest_memory,
                    core::slice::from_ref(&child_state),
                );
            }
        }
        // PR #25 (`feature/cache-shadow-page-table-state`) invalidates every
        // cached root conservatively; each root resynchronizes at most once per write.
        self.shadow_state.shadow_page_tables.record_pte_write();
        // PR #24 (`feature/shadow-paging-profile`) distinguishes incremental
        // updates from a fallback scan when a page did not have initialized state.
        self.shadow_state.shadow_paging_stats.record_pte_update(fallback_scan);
    }

}

#[inline]
fn read_cycle() -> usize {
    let cycles: usize;
    unsafe {
        core::arch::asm!("rdcycle {}", out(reg) cycles);
    }
    cycles
}
