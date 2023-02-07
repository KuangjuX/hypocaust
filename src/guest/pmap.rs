use alloc::collections::{VecDeque, BTreeMap};
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use crate::debug::PageDebug;
use crate::page_table::{PageTable, VirtPageNum, PageTableEntry, PhysPageNum, PTEFlags};
use crate::constants::layout::{GUEST_KERNEL_VIRT_START, TRAMPOLINE, TRAP_CONTEXT};
use crate::mm::KERNEL_SPACE;

use super::GuestKernel;

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

pub struct ShadowPageTables<P: PageTable + PageDebug> {
    /// all shadow page tables (satp, spt)
    pub spts: UnsafeCell<BTreeMap<usize, P>>,
    /// guest kernel installed shadow page table
    pub page_tables: [Option<usize>; 3],
    /// kernel guest page table token
    pub guest_satp: Option<usize>
}

impl<P> ShadowPageTables<P> where P: PageDebug + PageTable {
    pub const fn new() -> Self {
        Self {
            spts: UnsafeCell::new(BTreeMap::new()),
            page_tables: [None; 3],
            guest_satp: None
        }
    }

    pub fn spts(&self) -> &mut BTreeMap<usize, P> {
        unsafe{ &mut *self.spts.get() }
    }

    pub fn push(&self, satp: usize, spt: P) {
        let inner = self.spts();
        inner.insert(satp, spt);
    }


    pub fn shadow_page_table(&self, satp: usize) -> Option<&mut P> {
        let inner = self.spts();
        inner.get_mut(&satp)
    }

    pub fn guest_page_table(&self) -> Option<&mut P> {
        let inner = self.spts();
        if let Some(guest_satp) = self.guest_satp {
            inner.get_mut(&guest_satp)
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
        if pte.bits != 0 { drop = false; }
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

pub fn synchronize_page_table<P: PageTable>(hart_id: usize, satp: usize) {
    let guest_root_pa  = (satp & 0xfff_ffff_ffff) << 12;

    // 遍历所有页表项
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    let vpn = VirtPageNum::from(guest_root_pa >> 12);
    queue.push_back(vpn);

    for walk in 0..3 {
        // 遍历三级页表
        while !queue.is_empty() {
            // 获得 guest pte 的虚拟页号
            let guest_page_table_vpn = queue.pop_front().unwrap();
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
}

/// 用于初始化影子页表同步所有页表项(仅在最开始时使用)
pub fn initialize_shadow_page_table<P: PageTable>(hart_id: usize, satp: usize, mode: PageTableRoot, guest_spt: Option<&mut P>) -> Option<P> {
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
                    let host_pte = PageTableEntry::new(PhysPageNum::from(gpa2hpa(guest_pte.ppn().0 << 12, hart_id) >> 12) , guest_pte.flags() | PTEFlags::U);
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
    Some(host_shadow_page_table)
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
            let pte = spt.translate(vpn);
            pte
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
        // 根据 satp 获取 guest kernel 根页表的物理地址
        let hart_id = self.guest_id;
        let root_gpa = (satp & 0xfff_ffff_ffff) << 12;
        let root_hppn = PhysPageNum::from(gpa2hpa(root_gpa, hart_id) >> 12);
        let gpt = P::from_ppn(root_hppn);
        if self.shadow_state.shadow_page_tables.shadow_page_table(satp).is_none() {
            // 如果影子页表中没有发现，新建影子页表
            let mut spt;
            let mode;
            // 根据页表是否可读内核地址空间判断是 `GVA` 还是 `UVA`
            match page_table_mode(gpt.clone(), hart_id) {
                PageTableRoot::GVA => {
                    // 将 mode 设置为 `GVA`
                    mode = PageTableRoot::GVA;
                    spt = initialize_shadow_page_table::<P>(hart_id, satp, mode, None).unwrap();
                    self.shadow_state.shadow_page_tables.guest_satp = Some(satp);
                }
                PageTableRoot::UVA => {
                    // 将 mode 设置为 `UVA`
                    mode = PageTableRoot::UVA;
                    // 同步 guest spt,即将用户页表设置为只读
                    let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();   
                    spt = initialize_shadow_page_table::<P>(hart_id, satp, mode, Some(guest_spt)).unwrap();              
                    
                }
                _ => unreachable!()
            }

            // 为 `SPT` 映射跳板页
            // 无论是 guest spt 还是 user spt 都要映射跳板页与 Trap Context
            let trampoline_hppn = KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(TRAMPOLINE >> 12)).unwrap().ppn();
            spt.map(VirtPageNum::from(TRAMPOLINE >> 12), trampoline_hppn, PTEFlags::R | PTEFlags::X);

            let trapctx_hvpn = VirtPageNum::from(self.translate_guest_paddr(TRAP_CONTEXT).unwrap() >> 12);
            let trapctx_hppn = KERNEL_SPACE.exclusive_access().translate(trapctx_hvpn).unwrap().ppn();
            spt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);

            // hdebug!("Make new SPT(satp -> {:#x}, spt -> {:#x}) ", satp, spt.token());
            self.shadow_state.shadow_page_tables.install_root(spt.token(), mode);
            self.shadow_state.shadow_page_tables.push(satp, spt);
        }else{
            // 如果存在的话，根据 `guest page table` 更新 `guest os SPT` 只读项
            let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
            match page_table_mode(gpt.clone(), hart_id) {
                PageTableRoot::GVA => {
                    // os 的内存映射几乎不会改变,因此在切换页表时不需要同步
                    self.shadow_state.conseutive_satp_switch_count += 1;
                    // 切换的页表为 `guest os page table`
                    // 需要重新遍历所有页表项，并将其设置为只读
                    collect_page_table_vpns::<P>(hart_id, satp).iter().for_each(|&vpn| {
                        update_pte_readonly(vpn, guest_spt);
                    });
                },
                PageTableRoot::UVA => {
                    collect_page_table_vpns::<P>(hart_id, satp).iter().for_each(|&vpn| {
                        update_pte_readonly(vpn, guest_spt);
                    });
                    // 需要更新用户态页表
                    synchronize_page_table::<P>(hart_id, satp);
                    let spt = &mut self.shadow_state.shadow_page_tables.shadow_page_table(satp).unwrap();
                    // 为 `SPT` 映射跳板页
                    let trampoline_hppn = KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(TRAMPOLINE >> 12)).unwrap().ppn();
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
                    let trapctx_hppn = KERNEL_SPACE.exclusive_access().translate(trapctx_hvpn).unwrap().ppn();
                    if let Some(pte) = spt.translate(VirtPageNum::from(TRAP_CONTEXT >> 12)) {
                        if !pte.is_valid() {
                            htracking!("user remap trap context");
                            spt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);
                        }
                    }else{
                        htracking!("user remap trap context");
                        spt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);
                    }
                    self.shadow_state.shadow_page_tables.install_root(spt.token(), PageTableRoot::UVA);
                },
                _ => unreachable!()
            }
        }
    }



    pub fn synchronize_page_table(&mut self, va: usize, pte: PageTableEntry) {
        let hart_id = self.guest_id;
        // 获取对应影子页表的地址
        let host_pa = gpt2spt(va, hart_id);
        let host_ppn = PhysPageNum::from(host_pa >> 12);
        // 获得影子页表
        let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
        if va % core::mem::size_of::<PageTableEntry>() != 0 {
            panic!("Page Table Entry aligned?");
        }else if va % core::mem::size_of::<PageTableEntry>() == 0 && pte.bits == 0 {
            // 页表项对齐且物理页号为 0, 写入 `u8`
            unsafe{ core::ptr::write(host_pa as *mut usize, pte.bits as usize) };
            // 消除页表映射，将页表内存修改为可读可写
            clear_page_table(guest_spt, va, hart_id);
        }else {
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
            }else{
                unreachable!()
            }
        }
    }

}

