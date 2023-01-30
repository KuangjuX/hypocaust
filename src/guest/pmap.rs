use core::intrinsics::size_of;

use alloc::collections::{ VecDeque, BTreeMap };
use alloc::vec::Vec;
use riscv::addr::BitField;

use crate::debug::PageDebug;
use crate::mm::KERNEL_SPACE;
use crate::page_table::{PageTable, VirtPageNum, PTEFlags, PageTableEntry, PhysPageNum};
use crate::constants::layout::{ PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, GUEST_KERNEL_VIRT_START, GUEST_KERNEL_VIRT_END, GUEST_TRAMPOLINE, GUEST_TRAP_CONTEXT };
use super::GuestKernel;

/// 内存信息，用于帮助做地址映射
#[allow(unused)]
mod segment_layout {
    pub const HART_SEGMENT_SIZE: usize = 128 * 1024 * 1024;
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

pub struct Rmap {
    pub rmap: BTreeMap<PhysPageNum, usize>
}

pub struct ShadowPageTable<P: PageTable + PageDebug> {
    mode: PageTableRoot,
    /// 客户页表对应的 `satp`
    pub satp: usize,
    /// 影子页表
    pub page_table: P,
    /// 反向页表，用于从物理地址找回虚拟地址
    pub rmap: Option<Rmap>
}

impl<P> ShadowPageTable<P> where P: PageDebug + PageTable {
    pub fn new(satp: usize, page_table: P, rmap: Option<Rmap>, mode: PageTableRoot) -> Self {
        Self {
            mode,
            satp,
            page_table,
            rmap
        }
    }
}

/// 用来存放 Guest
pub struct ShadowPageTables<P: PageTable + PageDebug> {
    pub page_tables: VecDeque<ShadowPageTable<P>>
}

impl<P> ShadowPageTables<P> where P: PageDebug + PageTable {
    pub const fn new() -> Self {
        Self {
            page_tables: VecDeque::new()
        }
    }

    pub fn push(&mut self, shadow_page_table: ShadowPageTable<P>) {
        self.page_tables.push_back(shadow_page_table);
    }

    pub fn find_shadow_page_table(&self, satp: usize) -> Option<&ShadowPageTable<P>> {
        for item in self.page_tables.iter() {
            if item.satp == satp {
                return Some(item)
            }
        }
        None
    }

    pub fn find_shadow_page_table_mut(&mut self, satp: usize) -> Option<&mut ShadowPageTable<P>> {
        for item in self.page_tables.iter_mut() {
            if item.satp == satp {
                return Some(item)
            }
        }
        None
    }

    pub fn guest_page_table(&self) -> Option<&ShadowPageTable<P>> {
        for item in self.page_tables.iter() {
            if item.mode == PageTableRoot::GVA {
                return Some(item)
            }
        }
        None
    }

    pub fn guest_page_table_mut(&mut self) -> Option<&mut ShadowPageTable<P>> {
        for item in self.page_tables.iter_mut() {
            if item.mode == PageTableRoot::GVA {
                return Some(item)
            }
        }
        None
    }

}

pub fn gpa2hpa(va: usize, hart_id: usize) -> usize {
    va + (hart_id + 1) * segment_layout::HART_SEGMENT_SIZE
}

pub fn hpa2gpa(pa: usize, hart_id: usize) -> usize {
    pa - (hart_id + 1) * segment_layout::HART_SEGMENT_SIZE
}

pub fn try_map_guest_area<P: PageTable>(hart_id: usize, gpt: &P, spt: &mut P) {
    for gva in (GUEST_KERNEL_VIRT_START..GUEST_KERNEL_VIRT_END).step_by(PAGE_SIZE) {
        let gvpn = VirtPageNum::from(gva >> 12);
        let gppn = gpt.translate_gvpn(gvpn, hart_id);
        // 如果 guest ppn 存在且有效
        if let Some(gpte) = gppn {
            if gpte.is_valid() {
                let hppn = PhysPageNum::from(gpa2hpa(gpte.ppn().0 << 12, hart_id) >> 12);
                let mut pte_flags = PTEFlags::U;
                if gpte.readable() {
                    pte_flags |= PTEFlags::R;
                }
                if gpte.writable() {
                    pte_flags |= PTEFlags::W;
                }
                if gpte.executable() {
                    pte_flags |= PTEFlags::X;
                }
                spt.try_map(gvpn, hppn, pte_flags)
            }
        }
    }
}

fn try_map_user_area<P: PageTable>(hart_id: usize, guest_pgt: &P, shadow_pgt: &mut P) {
    for gva in (0x10000..0x80000).step_by(PAGE_SIZE) {
        let gvpn = VirtPageNum::from(gva >> 12);
        let gppn = guest_pgt.translate_gvpn(gvpn, hart_id);
        // 如果 guest ppn 存在且有效
        if let Some(gpte) = gppn {
            if gpte.is_valid() {
                let gpa = gpte.ppn().0 << 12;
                let hppn = PhysPageNum::from(gpa2hpa(gpa, hart_id) >> 12);
                let mut pte_flags = PTEFlags::U;
                if gpte.readable() {
                    pte_flags |= PTEFlags::R;
                }
                if gpte.writable() {
                    pte_flags |= PTEFlags::W;
                }
                if gpte.executable() {
                    pte_flags |= PTEFlags::X;
                }
                shadow_pgt.try_map(gvpn, hppn, pte_flags)
            }
        }
    }
}

fn try_map_user_trampoline<P: PageTable>(hart_id: usize, gpt: &P, spt: &mut P) {
    // 映射 guest 跳板页
    let guest_trampoline_gvpn = VirtPageNum::from(GUEST_TRAMPOLINE >> 12);
    if let Some(guest_trampoline_gpte) = gpt.translate_gvpn(guest_trampoline_gvpn, hart_id) {
        if guest_trampoline_gpte.is_valid() {
            let guest_trampoline_gppn = guest_trampoline_gpte.ppn();
            let guest_trampoline_hppn = PhysPageNum::from(gpa2hpa(guest_trampoline_gppn.0 << 12, hart_id) >> 12);
            spt.try_map(guest_trampoline_gvpn, guest_trampoline_hppn, PTEFlags::R | PTEFlags::X | PTEFlags::U);
        }
    }
}

fn try_map_user_trap_context<P: PageTable>(hart_id: usize, gpt: &P, spt: &mut P) {
    let guest_trap_context_gvpn = VirtPageNum::from(GUEST_TRAP_CONTEXT >> 12);
    if let Some(guest_trap_context_gpte) = gpt.translate_gvpn(guest_trap_context_gvpn, hart_id) {
        if guest_trap_context_gpte.is_valid() {
            let guest_trap_context_gppn = guest_trap_context_gpte.ppn();
            let guest_trap_context_hppn = PhysPageNum::from(gpa2hpa(guest_trap_context_gppn.0 << 12, hart_id) >> 12);
            spt.try_map(guest_trap_context_gvpn, guest_trap_context_hppn, PTEFlags::R | PTEFlags::W | PTEFlags::U);
        }
    }
}

/// 对于页表的 PTE 的标志位应当标志为只读，用来同步 Guest Page Table 和 Shadow Page Table
pub fn map_page_table<P: PageTable>(hart_id: usize, root_gpa: usize, spt: &mut P) {
    let root_gvpn = VirtPageNum::from(root_gpa >> 12);
    // 广度优先搜索遍历所有页表
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    queue.push_back(root_gvpn); 
    for _ in 0..=2 {
        while !queue.is_empty() {
            let vpn = queue.pop_front().unwrap();
            let ppn = PhysPageNum::from(gpa2hpa(vpn.0 << 12, hart_id) >> 12);
            // 如果 `spt` 中的该虚拟页已经被映射了，先解除再进行映射
            if let Some(pte) = spt.translate(vpn) {
                if pte.is_valid(){ spt.unmap(vpn); }
            }
            spt.map(vpn, ppn, PTEFlags::R | PTEFlags::U);
            let ptes = ppn.get_pte_array();
            for pte in ptes {
                if pte.is_valid() {
                    buffer.push(VirtPageNum::from(pte.ppn().0))
                }
            } 
        }
        while !buffer.is_empty() {
            queue.push_back(buffer.pop().unwrap());
        }
    }
}

/// 构造 `rmap`(反向页表映射)
pub fn make_rmap(hart_id: usize, root_gpa: usize, rmap: &mut Rmap) {
    let root_gvpn = VirtPageNum::from(root_gpa >> 12);
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    queue.push_back(root_gvpn); 
    for _ in 0..=1 {
        while !queue.is_empty() {
            let vpn = queue.pop_front().unwrap();
            let ppn = PhysPageNum::from(gpa2hpa(vpn.0 << 12, hart_id) >> 12);
            
            let ptes = ppn.get_pte_array();
            for (index, pte) in ptes.iter().enumerate() {
                if pte.is_valid() {
                    if !rmap.rmap.contains_key(&pte.ppn()) {
                        rmap.rmap.insert(pte.ppn(), (vpn.0 << 12) + index * size_of::<PageTableEntry>());
                        buffer.push(VirtPageNum::from(pte.ppn().0))
                    }else{ panic!("ppn exists: {:?}", pte.ppn()) }
                }
            } 
        }
        while !buffer.is_empty() {
            queue.push_back(buffer.pop().unwrap());
        }
    }
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
        self.memory.translate(vpn)
    }

    pub fn translate_guest_vpte(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        if let Some(spt) = self.shadow_state.shadow_page_tables.find_shadow_page_table(self.shadow_state.get_satp()) {
            // 由于 GHA 与 GPA 是同等映射的，因此翻译成的物理地址可以直接当虚拟地址用
            let pte = spt.page_table.translate(vpn);
            pte
        }else{
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

    /// 验证需要映射的内存是否为客户页表的页表项，若为页表项，则将
    /// 权限位设置为不可写，以便在 Guest OS 修改页表项时陷入 VMM
    pub fn is_guest_page_table(&self, vaddr: usize) -> bool {
        // 虚拟地址页对齐
        let satp = self.shadow_state.get_satp();
        if let Some(spt) = self.shadow_state.shadow_page_tables.find_shadow_page_table(satp) {
            if let Some(rmap) = &spt.rmap {
                if rmap.rmap.contains_key(&PhysPageNum::from(vaddr >> 12)) {
                    return true
                }
                return false
            }else{
                unimplemented!()
            }
        }
        false
    }

    /// 根据 satp 构建影子页表
    /// 需要将 GVA -> HPA
    pub fn make_shadow_page_table(&mut self, satp: usize) {
        if self.shadow_state.shadow_page_tables.find_shadow_page_table(satp).is_none() {
            // 如果影子页表中没有发现，新建影子页表
            // 根据 satp 获取 guest kernel 根页表的物理地址
            let hart_id = self.index;
            let root_gpa = (satp & 0xfff_ffff_ffff) << 12;
            let root_hppn = PhysPageNum::from(gpa2hpa(root_gpa, hart_id) >> 12);
            let gpt = P::from_ppn(root_hppn);
            // 翻译的时候不能直接翻译，因为此时取出的 pte 都是 Guest OS 的物理地址，需要将 pte 翻译成 hypervisor 的地址
            // 即将 guest virtual address -> host virtual address
            // 最终翻译的结果为 GPA (Guest Physical Address)
            // 构建影子页表
            let mut spt = PageTable::new();
            let mode;
            let guest_rmap;
            // 根据页表是否可读内核地址空间判断是 `GVA` 还是 `UVA`
            match page_table_mode(gpt.clone(), hart_id) {
                PageTableRoot::GVA => {
                    // 映射客户页表
                    map_page_table(hart_id, root_gpa, &mut spt);
                    // 尝试映射内核地址空间
                    try_map_guest_area(hart_id, &gpt, &mut spt);
                    // 尝试映射用户空间的跳板页
                    try_map_user_trampoline(hart_id, &gpt, &mut spt);
                    // 尝试映射用户空间 Trap Context
                    try_map_user_trap_context(hart_id, &gpt, &mut spt);
                    // 构造 `rmap`
                    let mut rmap = Rmap{ rmap: BTreeMap::new() };
                    make_rmap(hart_id, root_gpa, &mut rmap);
                    guest_rmap = Some(rmap);
                    // 将 mode 设置为 `GVA`
                    mode = PageTableRoot::GVA;

                }
                PageTableRoot::UVA => {
                    // 尝试映射用户地址空间
                    try_map_user_area(hart_id, &gpt, &mut spt);
                    // 尝试映射用户空间的跳板页
                    try_map_user_trampoline(hart_id, &gpt, &mut spt);
                    // 尝试映射用户空间 Trap Context
                    try_map_user_trap_context(hart_id, &gpt, &mut spt);
                    // 将 `guest_rmap` 设置为 `None`
                    guest_rmap = None;
                    // 将 mode 设置为 `UVA`
                    mode = PageTableRoot::UVA;
                    // 同步 guest spt,即将用户页表设置为只读
                    let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table_mut().unwrap();                   
                    // 将映射的页表项加入反向页表
                    make_rmap(hart_id, root_gpa, guest_spt.rmap.as_mut().unwrap());
                    // 将用户页表设置为只读
                    map_page_table(hart_id, root_gpa, &mut guest_spt.page_table);
                    
                }
                _ => unreachable!()
            }

            // 无论是 guest spt 还是 user spt 都要映射跳板页与 Trap Context
            let trampoline_hppn = KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(TRAMPOLINE >> 12)).unwrap().ppn();
            spt.map(VirtPageNum::from(TRAMPOLINE >> 12), trampoline_hppn, PTEFlags::R | PTEFlags::X);

            let trapctx_hvpn = VirtPageNum::from(self.translate_guest_paddr(TRAP_CONTEXT).unwrap() >> 12);
            let trapctx_hppn = KERNEL_SPACE.exclusive_access().translate(trapctx_hvpn).unwrap().ppn();
            spt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);

            // 修改 `shadow page table`
            // hdebug!("Make new SPT(satp -> {:#x}, spt -> {:#x}) ", satp, shadow_pgt.token());
            let shadow_page_table = ShadowPageTable::new(satp, spt, guest_rmap, mode);
            self.shadow_state.shadow_page_tables.push(shadow_page_table);
        }
    }

    /// 同步 `shadow page table` & `guest page table`
    pub fn sync_shadow_page_table(&mut self, mut vaddr: usize, pte: PageTableEntry) {
        let satp = self.shadow_state.get_satp();
        let root_ppn = PhysPageNum::from(satp & 0xfff_ffff_ffff);
        let hart_id = self.index;
        if let Some(spt) = self.shadow_state.shadow_page_tables.find_shadow_page_table_mut(satp) {
            if let Some(rmap) = &mut spt.rmap {
                let mut map_vpn: usize = 0;
                let mut ppn = PhysPageNum::from(vaddr >> 12);
                let mut i = 0;
                while ppn != root_ppn {
                    let index = (vaddr & 0xfff) / size_of::<PageTableEntry>();
                    map_vpn.set_bits((i * 9)..(i * 9) + 9, index);
                    if let Some(value) = rmap.rmap.get(&ppn) {
                        vaddr = *value;
                        ppn = PhysPageNum::from(vaddr >> 12);
                        i += 1;
                    }else{
                        break;
                    }
                }
                if i == 2 {
                    // 生成虚拟页号
                    let index = (vaddr & 0xfff) / size_of::<PageTableEntry>();
                    map_vpn.set_bits((i * 9)..(i * 9) + 9, index);
                    let map_vpn = VirtPageNum::from(map_vpn);
                    if pte.is_valid() {
                        let mut flags = PTEFlags::U;
                        if pte.readable(){ flags |= PTEFlags::R };
                        if pte.writable(){ flags |= PTEFlags::W };
                        if pte.executable(){ flags |= PTEFlags::X };
                        let pa = gpa2hpa(pte.ppn().0 << 12, hart_id);
                        let ppn = PhysPageNum::from(pa >> 12);
                        spt.page_table.map(map_vpn, ppn, flags);
                    }else{
                        spt.page_table.unmap(map_vpn);
                        rmap.rmap.remove(&PhysPageNum::from(vaddr >> 12));
                    }
                }else {
                    unimplemented!()
                }
            }else{
                unimplemented!()
            }
            
        }
     }

}

pub fn page_table_mode<P: PageTable>(page_table: P, hart_id: usize) -> PageTableRoot {
    if page_table.translate_gvpn(VirtPageNum::from(GUEST_KERNEL_VIRT_START >> 12), hart_id).is_some() {
        return PageTableRoot::GVA
    }
    PageTableRoot::UVA
}