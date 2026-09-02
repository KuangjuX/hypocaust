use alloc::collections::{VecDeque, BTreeMap};
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use crate::debug::PageDebug;
use crate::device_emu::is_device_access;
use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::page_table::{PageTable, VirtPageNum, PageTableEntry, PhysPageNum, PTEFlags};
use crate::constants::layout::{GUEST_KERNEL_VIRT_START, TRAMPOLINE, TRAP_CONTEXT};

use super::GuestKernel;
use super::shadow_stats::ShadowPageTableUpdate;

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

struct CachedShadowPageTable<P: PageTable + PageDebug> {
    page_table: P,
    synchronized_generation: usize,
}

pub struct ShadowPageTables<P: PageTable + PageDebug> {
    /// all shadow page tables (satp, spt)
    spts: UnsafeCell<BTreeMap<usize, CachedShadowPageTable<P>>>,
    /// guest kernel installed shadow page table
    pub page_tables: [Option<usize>; 3],
    /// kernel guest page table token
    pub guest_satp: Option<usize>,
    /// `feature/cache-shadow-page-table-state` increments this generation for
    /// every trapped guest PTE write so cached roots can detect stale state.
    pte_generation: usize,
}

impl<P> ShadowPageTables<P> where P: PageDebug + PageTable {
    pub const fn new() -> Self {
        Self {
            spts: UnsafeCell::new(BTreeMap::new()),
            page_tables: [None; 3],
            guest_satp: None,
            pte_generation: 0,
        }
    }

    fn spts(&self) -> &mut BTreeMap<usize, CachedShadowPageTable<P>> {
        unsafe{ &mut *self.spts.get() }
    }

    pub fn push(&self, satp: usize, spt: P) {
        let inner = self.spts();
        inner.insert(satp, CachedShadowPageTable {
            page_table: spt,
            synchronized_generation: self.pte_generation,
        });
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

pub fn gpa2hpa(va: usize, hart_id: usize) -> usize {
    va + (hart_id + 1) * segment_layout::HART_SEGMENT_SIZE
}

pub fn hpa2gpa(pa: usize, hart_id: usize) -> usize {
    pa - (hart_id + 1) * segment_layout::HART_SEGMENT_SIZE
}

pub fn gpt2spt(va: usize, hart_id: usize) -> usize {
    va + segment_layout::SPT_OFFSET + hart_id * segment_layout::HART_SEGMENT_SIZE
}

pub fn page_table_mode<P: PageTable>(page_table: P, hart_id: usize) -> PageTableRoot {
    if page_table.translate_guest(VirtPageNum::from(GUEST_KERNEL_VIRT_START >> 12), hart_id).is_some() {
        return PageTableRoot::GVA
    }
    PageTableRoot::UVA
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

fn clear_page_table<P: PageTable>(spt: &mut P, va: usize, hart_id: usize) {
    let mut drop = true;
    let guest_ppn = PhysPageNum::from(gpa2hpa(va, hart_id) >> 12);
    let guest_ptes = guest_ppn.get_pte_array();
    guest_ptes.iter().for_each(|&pte| {
        // PR #21 (fix-bug/invalid-pte-synchronization): software may retain metadata
        // in an invalid PTE, so only V=1 keeps the page protected as a page table.
        if pte.is_valid() { drop = false; }
    });
    if drop {
        // htracking!("Drop the page table guest ppn -> {:#x}", guest_ppn.0);
        // 将影子页表设置为可读可写
        if let Some(spt_pte) = spt.find_pte(VirtPageNum::from(va >> 12)) {
            *spt_pte = PageTableEntry::new(spt_pte.ppn(), PTEFlags::R | PTEFlags::W | PTEFlags::U | PTEFlags::V);
        }
    }
}

/// 收集所有页表的虚拟页号
pub fn collect_page_table_vpns<P: PageTable>(hart_id: usize, satp: usize) -> Vec<VirtPageNum> {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;

    // 遍历所有页表项
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    // 非叶子所在的虚拟页号
    let mut non_leaf_vpns = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back(vpn);

    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let guest_page_table_vpn = queue.pop_front().unwrap();
            // 收集所有非叶子节点 `vpn`，用于设置为只读
            non_leaf_vpns.push(guest_page_table_vpn);
            // 获得 guest pte 的物理页号
            let guest_page_table_ppn = PhysPageNum::from(gpa2hpa(guest_page_table_vpn.0 << 12, hart_id) >> 12);
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            for guest_pte in guest_ptes.iter(){
                if guest_pte.is_valid() && walk < 2 {
                    // 非叶子页表项
                    buffer.push(VirtPageNum::from(guest_pte.ppn().0));
                }else if guest_pte.is_valid() && walk == 2 {
                }
            }
        }
        while !buffer.is_empty() {
            queue.push_back(buffer.pop().unwrap());
        }
    }
    non_leaf_vpns
    
}

pub fn synchronize_page_table<P: PageTable>(hart_id: usize, satp: usize) -> usize {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;

    // 遍历所有页表项
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back(vpn);
    let mut walked_page_table_pages = 0;

    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let guest_page_table_vpn = queue.pop_front().unwrap();
            walked_page_table_pages += 1;
            // 收集所有非叶子节点 `vpn`，用于设置为只读
            let host_page_table_ppn = PhysPageNum::from(gpt2spt(guest_page_table_vpn.0 << 12, hart_id) >> 12);
            // 获得 guest pte 的物理页号
            let guest_page_table_ppn = PhysPageNum::from(gpa2hpa(guest_page_table_vpn.0 << 12, hart_id) >> 12);
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            // 获得 host pte 页表项内容
            let host_ptes = host_page_table_ppn.get_pte_array();
            for (index, guest_pte) in guest_ptes.iter().enumerate() {
                if guest_pte.is_valid() && walk < 2 {
                    // 非叶子页表项
                    buffer.push(VirtPageNum::from(guest_pte.ppn().0));
                    // 构造 host pte
                    let host_pte = PageTableEntry::new(PhysPageNum::from(gpt2spt(guest_pte.ppn().0 << 12, hart_id) >> 12) , guest_pte.flags());
                    host_ptes[index] = host_pte;
                }else if guest_pte.is_valid() && walk == 2 {
                    let host_pte = PageTableEntry::new(PhysPageNum::from(gpa2hpa(guest_pte.ppn().0 << 12, hart_id) >> 12) , guest_pte.flags() | PTEFlags::U);
                    host_ptes[index] = host_pte;
                }
            }
        }
        while !buffer.is_empty() {
            queue.push_back(buffer.pop().unwrap());
        }
    }
    walked_page_table_pages
}

/// 用于初始化影子页表同步所有页表项(仅在最开始时使用)
pub fn initialize_shadow_page_table<P: PageTable>(hart_id: usize, satp: usize, mode: PageTableRoot, guest_spt: Option<&mut P>) -> Option<(P, usize)> {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;
    let host_root_pa = gpt2spt(guest_root_pa, hart_id);
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
    let mut non_leaf_vpns = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back(vpn);
    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let guest_page_table_vpn = queue.pop_front().unwrap();
            // 收集所有非叶子节点 `vpn`，用于设置为只读
            non_leaf_vpns.push(guest_page_table_vpn);
            let host_page_table_ppn = PhysPageNum::from(gpt2spt(guest_page_table_vpn.0 << 12, hart_id) >> 12);
            // 获得 guest pte 的物理页号
            let guest_page_table_ppn = PhysPageNum::from(gpa2hpa(guest_page_table_vpn.0 << 12, hart_id) >> 12);
            // 获得 guest pte 页表项内容
            let guest_ptes = guest_page_table_ppn.get_pte_array();
            // 获得 host pte 页表项内容
            let host_ptes = host_page_table_ppn.get_pte_array();
            for (index, guest_pte) in guest_ptes.iter().enumerate() {
                if guest_pte.is_valid() && walk < 2 {
                    // 非叶子页表项
                    buffer.push(VirtPageNum::from(guest_pte.ppn().0));
                    // 构造 host pte
                    let host_pte = PageTableEntry::new(PhysPageNum::from(gpt2spt(guest_pte.ppn().0 << 12, hart_id) >> 12) , guest_pte.flags());
                    host_ptes[index] = host_pte;
                }else if guest_pte.is_valid() && walk == 2 {
                    let host_pte;
                    if !is_device_access(guest_pte.ppn().0 << 12) {
                        host_pte = PageTableEntry::new(PhysPageNum::from(gpa2hpa(guest_pte.ppn().0 << 12, hart_id) >> 12) , guest_pte.flags() | PTEFlags::U);
                    }else{
                        // PR #17 (fix-bug/virtio-dma-translation): leave passthrough
                        // devices unmapped so MMIO traps before reaching QEMU.
                        host_pte = PageTableEntry::empty();
                    }
                    host_ptes[index] = host_pte;
                }
            }
        }
        while !buffer.is_empty() {
            queue.push_back(buffer.pop().unwrap());
        }
    }
    let mut host_shadow_page_table = PageTable::from_ppn(PhysPageNum::from(host_root_pa >> 12));
    non_leaf_vpns.iter().for_each(|&vpn| {
        match mode {
            PageTableRoot::GVA => {
                update_pte_readonly(vpn, &mut host_shadow_page_table);
            },
            PageTableRoot::UVA => {
                update_pte_readonly(vpn, guest_spt);
            },
            _ => unreachable!()
        }
    });
    let walked_page_table_pages = non_leaf_vpns.len();
    Some((host_shadow_page_table, walked_page_table_pages))
}




impl<P> GuestKernel<P> where P: PageDebug + PageTable {
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
        let hart_id = self.guest_id;
        // Preserve the original mode classification on every switch. A guest
        // may reuse a root PPN, so only synchronization state is cached.
        let root_gpa = (satp & 0xfff_ffff_ffff) << 12;
        let root_hppn = PhysPageNum::from(gpa2hpa(root_gpa, hart_id) >> 12);
        let gpt = P::from_ppn(root_hppn);
        let mode = page_table_mode(gpt, hart_id);
        let requires_resynchronization = self.shadow_state.shadow_page_tables
            .requires_resynchronization(satp);
        if requires_resynchronization.is_none() {
            update = ShadowPageTableUpdate::New;
            // 如果影子页表中没有发现，新建影子页表
            let mut spt;
            // 根据页表是否可读内核地址空间判断是 `GVA` 还是 `UVA`
            match mode {
                PageTableRoot::GVA => {
                    let initialized = initialize_shadow_page_table::<P>(hart_id, satp, mode, None).unwrap();
                    spt = initialized.0;
                    walked_page_table_pages += initialized.1;
                    full_walks += 1;
                    self.shadow_state.shadow_page_tables.guest_satp = Some(satp);

                    assert!(!spt.translate(VirtPageNum::from(0x10001)).unwrap().is_valid());
                }
                PageTableRoot::UVA => {
                    // 同步 guest spt,即将用户页表设置为只读
                    let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();   
                    let initialized = initialize_shadow_page_table::<P>(hart_id, satp, mode, Some(guest_spt)).unwrap();
                    spt = initialized.0;
                    walked_page_table_pages += initialized.1;
                    full_walks += 1;
                    
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
            self.shadow_state.shadow_page_tables.install_root(spt.token(), mode);
            self.shadow_state.shadow_page_tables.push(satp, spt);
        }else{
            let requires_resynchronization = requires_resynchronization.unwrap();
            match mode {
                PageTableRoot::GVA => {
                    update = ShadowPageTableUpdate::CachedKernel;
                    // os 的内存映射几乎不会改变,因此在切换页表时不需要同步
                    self.shadow_state.conseutive_satp_switch_count += 1;
                    if requires_resynchronization {
                        // `feature/cache-shadow-page-table-state` only revisits
                        // cached page-table pages after a guest PTE was written.
                        let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
                        let page_table_vpns = collect_page_table_vpns::<P>(hart_id, satp);
                        walked_page_table_pages += page_table_vpns.len();
                        full_walks += 1;
                        page_table_vpns.iter().for_each(|&vpn| {
                            update_pte_readonly(vpn, guest_spt);
                        });
                        self.shadow_state.shadow_page_tables.mark_synchronized(satp);
                    }
                },
                PageTableRoot::UVA => {
                    update = ShadowPageTableUpdate::CachedUser;
                    if requires_resynchronization {
                        let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
                        let page_table_vpns = collect_page_table_vpns::<P>(hart_id, satp);
                        walked_page_table_pages += page_table_vpns.len();
                        full_walks += 1;
                        page_table_vpns.iter().for_each(|&vpn| {
                            update_pte_readonly(vpn, guest_spt);
                        });
                        // 需要更新用户态页表
                        walked_page_table_pages += synchronize_page_table::<P>(hart_id, satp);
                        full_walks += 1;
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
                    let spt = &mut self.shadow_state.shadow_page_tables.shadow_page_table(satp).unwrap();
                    self.shadow_state.shadow_page_tables.install_root(spt.token(), PageTableRoot::UVA);
                },
                _ => unreachable!()
            }
        }
        let elapsed_cycles = read_cycle().wrapping_sub(start_cycles);
        self.shadow_state.shadow_paging_stats.record_satp_update(
            update,
            full_walks,
            walked_page_table_pages,
            elapsed_cycles,
        );
    }



    pub fn synchronize_page_table(&mut self, va: usize, pte: PageTableEntry) {
        let hart_id = self.guest_id;
        // 获取对应影子页表的地址
        let host_pa = gpt2spt(va, hart_id);
        let host_ppn = PhysPageNum::from(host_pa >> 12);
        // 获得影子页表
        let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
        let invalidation_scan;
        if va % core::mem::size_of::<PageTableEntry>() != 0 {
            panic!("Page Table Entry aligned?");
        }else if va % core::mem::size_of::<PageTableEntry>() == 0 && !pte.is_valid() {
            invalidation_scan = true;
            // PR #21 (fix-bug/invalid-pte-synchronization): mirror every V=0 encoding,
            // including allocator metadata, and release pages with no valid PTEs.
            unsafe{ core::ptr::write(host_pa as *mut usize, pte.bits as usize) };
            // 消除页表映射，将页表内存修改为可读可写
            clear_page_table(guest_spt, va, hart_id);
        }else {
            invalidation_scan = false;
            // 如果页表项对齐且物理页号不为零表示进行页表映射
            let index = (host_pa & 0xfff) / core::mem::size_of::<PageTableEntry>();
            let pte_array = host_ppn.get_pte_array();
            if pte.is_valid() && (pte.readable() | pte.writable() | pte.executable()) {
                // 叶子节点
                let new_ppn = PhysPageNum::from(gpa2hpa(pte.ppn().0 << 12, hart_id) >> 12);
                let new_flags = pte.flags() | PTEFlags::U;
                let new_pte = PageTableEntry::new(new_ppn, new_flags);
                pte_array[index] = new_pte;
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
                let new_ppn = PhysPageNum::from(gpt2spt(pte.ppn().0 << 12, hart_id) >> 12);
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
        // `feature/cache-shadow-page-table-state` invalidates every cached
        // root conservatively; each root resynchronizes at most once per write.
        self.shadow_state.shadow_page_tables.record_pte_write();
        // PR #24 (`feature/shadow-paging-profile`) distinguishes incremental
        // updates from the 512-entry scan performed when a PTE is invalidated.
        self.shadow_state.shadow_paging_stats.record_pte_update(invalidation_scan);
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
