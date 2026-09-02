use alloc::collections::{VecDeque, BTreeMap};
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use crate::debug::PageDebug;
use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::identity::VmId;
use crate::page_table::{PageTable, VirtPageNum, PageTableEntry, PhysPageNum, PTEFlags};
use crate::constants::layout::{
    GUEST_KERNEL_VIRT_END, GUEST_KERNEL_VIRT_START, PAGE_SIZE, TRAMPOLINE,
    TRAP_CONTEXT,
};

use super::Vcpu;
use super::shadow_stats::ShadowPageTableUpdate;

// PR #26 (`feature/shadow-page-table-asid`) encodes stable shadow-root ASIDs
// in the architectural RV64 Sv39 field while reserving ASID 0 for the Host.
const SATP_ASID_SHIFT: usize = 44;
const MAX_SV39_ASID: usize = (1 << 16) - 1;

/// 内存信息，用于帮助做地址映射
#[allow(unused)]
mod segment_layout {
    pub const HART_SEGMENT_SIZE: usize = 128 * 1024 * 1024;
    pub const SPT_OFFSET: usize = 0x10000_0000 - 0x8000_0000;
}



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

/// PR #34 (`feature/vm-vcpu-identities`) keys Guest-physical memory by VM ownership;
/// the physical hart running a vCPU must never change its translation.
pub fn gpa2hpa(va: usize, vm_id: VmId) -> usize {
    va + (vm_id.index() + 1) * segment_layout::HART_SEGMENT_SIZE
}

pub fn hpa2gpa(pa: usize, vm_id: VmId) -> usize {
    pa - (vm_id.index() + 1) * segment_layout::HART_SEGMENT_SIZE
}

pub fn gpt2spt(va: usize, vm_id: VmId) -> usize {
    va + segment_layout::SPT_OFFSET + vm_id.index() * segment_layout::HART_SEGMENT_SIZE
}

pub fn page_table_mode<P: PageTable>(page_table: P, vm_id: VmId) -> PageTableRoot {
    if page_table.translate_guest(VirtPageNum::from(GUEST_KERNEL_VIRT_START >> 12), vm_id).is_some() {
        return PageTableRoot::GVA
    }
    PageTableRoot::UVA
}

/// PR #35 (`fix-bug/non-ram-shadow-leaves`) maps only Guest RAM into shadow
/// leaves. MMIO and other non-RAM GPAs remain invalid and trap to Hypocaust.
fn shadow_leaf_pte(guest_pte: PageTableEntry, vm_id: VmId) -> PageTableEntry {
    let guest_page = guest_pte.ppn().0 << 12;
    let page_in_ram = guest_page >= GUEST_KERNEL_VIRT_START
        && guest_page
            .checked_add(PAGE_SIZE)
            .map_or(false, |end| end <= GUEST_KERNEL_VIRT_END);
    if page_in_ram {
        PageTableEntry::new(
            PhysPageNum::from(gpa2hpa(guest_page, vm_id) >> 12),
            guest_pte.flags() | PTEFlags::U,
        )
    } else {
        PageTableEntry::empty()
    }
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
    vm_id: VmId,
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
            let guest_page_table_ppn = PhysPageNum::from(gpa2hpa(guest_page_table_vpn.0 << 12, vm_id) >> 12);
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            let mut valid_pte_count = 0;
            for guest_pte in guest_ptes.iter(){
                if guest_pte.is_valid() {
                    valid_pte_count += 1;
                }
                if guest_pte.is_valid() && walk < 2 {
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
    vm_id: VmId,
    satp: usize,
) -> Vec<GuestPageTablePageState> {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;

    // 遍历所有页表项
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back(vpn);
    let mut page_table_pages = Vec::new();

    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let guest_page_table_vpn = queue.pop_front().unwrap();
            // 收集所有非叶子节点 `vpn`，用于设置为只读
            let host_page_table_ppn = PhysPageNum::from(gpt2spt(guest_page_table_vpn.0 << 12, vm_id) >> 12);
            // 获得 guest pte 的物理页号
            let guest_page_table_ppn = PhysPageNum::from(gpa2hpa(guest_page_table_vpn.0 << 12, vm_id) >> 12);
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            // 获得 host pte 页表项内容
            let host_ptes = host_page_table_ppn.get_pte_array();
            let mut valid_pte_count = 0;
            for (index, guest_pte) in guest_ptes.iter().enumerate() {
                if guest_pte.is_valid() {
                    valid_pte_count += 1;
                }
                if guest_pte.is_valid() && walk < 2 {
                    // 非叶子页表项
                    buffer.push(VirtPageNum::from(guest_pte.ppn().0));
                    // 构造 host pte
                    let host_pte = PageTableEntry::new(PhysPageNum::from(gpt2spt(guest_pte.ppn().0 << 12, vm_id) >> 12) , guest_pte.flags());
                    host_ptes[index] = host_pte;
                }else if guest_pte.is_valid() && walk == 2 {
                    host_ptes[index] = shadow_leaf_pte(*guest_pte, vm_id);
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

/// 用于初始化影子页表同步所有页表项(仅在最开始时使用)
fn initialize_shadow_page_table<P: PageTable>(
    vm_id: VmId,
    satp: usize,
    mode: PageTableRoot,
    guest_spt: Option<&mut P>,
) -> Option<(P, Vec<GuestPageTablePageState>)> {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;
    let host_root_pa = gpt2spt(guest_root_pa, vm_id);
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
    // 非叶子所在的虚拟页号
    let mut page_table_pages = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back(vpn);
    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let guest_page_table_vpn = queue.pop_front().unwrap();
            let host_page_table_ppn = PhysPageNum::from(gpt2spt(guest_page_table_vpn.0 << 12, vm_id) >> 12);
            // 获得 guest pte 的物理页号
            let guest_page_table_ppn = PhysPageNum::from(gpa2hpa(guest_page_table_vpn.0 << 12, vm_id) >> 12);
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            // 获得 host pte 页表项内容
            let host_ptes = host_page_table_ppn.get_pte_array();
            let mut valid_pte_count = 0;
            for (index, guest_pte) in guest_ptes.iter().enumerate() {
                if guest_pte.is_valid() {
                    valid_pte_count += 1;
                }
                if guest_pte.is_valid() && walk < 2 {
                    // 非叶子页表项
                    buffer.push(VirtPageNum::from(guest_pte.ppn().0));
                    // 构造 host pte
                    let host_pte = PageTableEntry::new(PhysPageNum::from(gpt2spt(guest_pte.ppn().0 << 12, vm_id) >> 12) , guest_pte.flags());
                    host_ptes[index] = host_pte;
                }else if guest_pte.is_valid() && walk == 2 {
                    host_ptes[index] = shadow_leaf_pte(*guest_pte, vm_id);
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
        let vm_id = self.vm_id;
        // PR #25 (`feature/cache-shadow-page-table-state`) still classifies the
        // live root because a guest may reuse a root PPN; only freshness is cached.
        let root_gpa = (satp & 0xfff_ffff_ffff) << 12;
        let root_hppn = PhysPageNum::from(gpa2hpa(root_gpa, vm_id) >> 12);
        let gpt = P::from_ppn(root_hppn);
        let mode = page_table_mode(gpt, vm_id);
        let requires_resynchronization = self.shadow_state.shadow_page_tables
            .requires_resynchronization(satp);
        if requires_resynchronization.is_none() {
            update = ShadowPageTableUpdate::New;
            // 如果影子页表中没有发现，新建影子页表
            let mut spt;
            // 根据页表是否可读内核地址空间判断是 `GVA` 还是 `UVA`
            match mode {
                PageTableRoot::GVA => {
                    let initialized = initialize_shadow_page_table::<P>(vm_id, satp, mode, None).unwrap();
                    spt = initialized.0;
                    walked_page_table_pages += initialized.1.len();
                    full_walks += 1;
                    self.shadow_state.shadow_page_tables.record_page_table_pages(&initialized.1);
                    self.shadow_state.shadow_page_tables.guest_satp = Some(satp);

                    assert!(!spt.translate(VirtPageNum::from(0x10001)).unwrap().is_valid());
                }
                PageTableRoot::UVA => {
                    // 同步 guest spt,即将用户页表设置为只读
                    let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();   
                    let initialized = initialize_shadow_page_table::<P>(vm_id, satp, mode, Some(guest_spt)).unwrap();
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
            let trampoline_hppn = hypervisor_memory.translate(VirtPageNum::from(TRAMPOLINE >> 12)).unwrap().ppn();
            spt.map(VirtPageNum::from(TRAMPOLINE >> 12), trampoline_hppn, PTEFlags::R | PTEFlags::X);

            let trapctx_hvpn = VirtPageNum::from(self.translate_guest_paddr(TRAP_CONTEXT).unwrap() >> 12);
            let trapctx_hppn = hypervisor_memory.translate(trapctx_hvpn).unwrap().ppn();
            spt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);

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
                        let page_table_pages = collect_page_table_pages::<P>(vm_id, satp);
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
                        let page_table_pages = collect_page_table_pages::<P>(vm_id, satp);
                        walked_page_table_pages += page_table_pages.len();
                        full_walks += 1;
                        page_table_pages.iter().for_each(|page| {
                            update_pte_readonly(page.vpn, guest_spt);
                        });
                        self.shadow_state.shadow_page_tables.record_page_table_pages(&page_table_pages);
                        // 需要更新用户态页表
                        let synchronized_pages = synchronize_page_table::<P>(vm_id, satp);
                        walked_page_table_pages += synchronized_pages.len();
                        full_walks += 1;
                        self.shadow_state.shadow_page_tables.record_page_table_pages(&synchronized_pages);
                        let spt = &mut self.shadow_state.shadow_page_tables.shadow_page_table(satp).unwrap();
                        let hypervisor_memory = HYPERVISOR_MEMORY.exclusive_access();
                        // 为 `SPT` 映射跳板页
                        let trampoline_hppn = hypervisor_memory.translate(VirtPageNum::from(TRAMPOLINE >> 12)).unwrap().ppn();
                        if let Some(pte) = spt.translate(VirtPageNum::from(TRAMPOLINE >> 12)) {
                            if !pte.is_valid() {
                                htracking!("user remap trampoline");
                                spt.map(VirtPageNum::from(TRAMPOLINE >> 12), trampoline_hppn, PTEFlags::R | PTEFlags::X);
                            }
                        }else{
                            htracking!("user remap trampoline");
                            spt.map(VirtPageNum::from(TRAMPOLINE >> 12), trampoline_hppn, PTEFlags::R | PTEFlags::X);
                        }

                        let trapctx_hvpn = VirtPageNum::from(self.translate_guest_paddr(TRAP_CONTEXT).unwrap() >> 12);
                        let trapctx_hppn = hypervisor_memory.translate(trapctx_hvpn).unwrap().ppn();
                        if let Some(pte) = spt.translate(VirtPageNum::from(TRAP_CONTEXT >> 12)) {
                            if !pte.is_valid() {
                                htracking!("user remap trap context");
                                spt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);
                            }
                        }else{
                            htracking!("user remap trap context");
                            spt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);
                        }
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
        let vm_id = self.vm_id;
        // 获取对应影子页表的地址
        let host_pa = gpt2spt(va, vm_id);
        let host_ppn = PhysPageNum::from(host_pa >> 12);
        if va % core::mem::size_of::<PageTableEntry>() != 0 {
            panic!("Page Table Entry aligned?");
        }
        let page_vpn = VirtPageNum::from(va >> 12);
        let (valid_pte_count, fallback_scan) = self
            .shadow_state
            .shadow_page_tables
            .update_valid_pte_count(page_vpn, old_pte, pte, || {
                let guest_ppn = PhysPageNum::from(gpa2hpa(va, vm_id) >> 12);
                guest_ppn
                    .get_pte_array()
                    .iter()
                    .filter(|pte| pte.is_valid())
                    .count()
            });
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
                pte_array[index] = shadow_leaf_pte(pte, vm_id);
                let vpn = VirtPageNum::from(va >> 12);
                if let Some(pte) = guest_spt.translate(vpn) {
                    if pte.writable() | pte.executable() {
                        htracking!("Allocate page table, ppn: {:#x}", vpn.0);
                        update_pte_readonly(vpn, guest_spt);
                    }
                }else{
                    panic!()
                }

            }else if pte.is_valid() && !(pte.readable() | pte.writable() | pte.executable()) {
                // 非叶子节点
                // 获取非叶子节点的偏移
                let new_ppn = PhysPageNum::from(gpt2spt(pte.ppn().0 << 12, vm_id) >> 12);
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
                }else{
                    unreachable!()
                }
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
