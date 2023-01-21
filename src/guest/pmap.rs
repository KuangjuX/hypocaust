use alloc::collections::VecDeque;
use alloc::vec::Vec;

use crate::mm::{PageTable, KERNEL_SPACE, VirtPageNum, PTEFlags, PageTableEntry, PhysPageNum};
use crate::constants::layout::{ PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, GUEST_KERNEL_VIRT_START_1, GUEST_KERNEL_VIRT_END_1, GUEST_TRAMPOLINE, GUEST_TRAP_CONTEXT };
use crate::board::{ QEMU_VIRT_START, QEMU_VIRT_SIZE };
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

pub struct ShadowPageTable {
    // mode: PageTableRoot,
    satp: usize,
    page_table: PageTable
}

impl ShadowPageTable {
    pub fn new(satp: usize, page_table: PageTable) -> Self {
        Self {
            // mode,
            satp,
            page_table
        }
    }
}

/// 用来存放 Guest
pub struct ShadowPageTables {
    page_tables: VecDeque<ShadowPageTable>
}

impl ShadowPageTables {
    pub const fn new() -> Self {
        Self {
            page_tables: VecDeque::new()
        }
    }

    pub fn push(&mut self, shadow_page_table: ShadowPageTable) {
        self.page_tables.push_back(shadow_page_table);
    }

    pub fn find_shadow_page_table(&self, satp: usize) -> Option<&PageTable> {
        for item in self.page_tables.iter() {
            if item.satp == satp {
                return Some(&item.page_table)
            }
        }
        None
    }

    pub fn find_shadow_page_table_mut(&mut self, satp: usize) -> Option<&mut PageTable> {
        for item in self.page_tables.iter_mut() {
            if item.satp == satp {
                return Some(&mut item.page_table)
            }
        }
        None
    }

}

impl GuestKernel {
    pub fn gpa2hpa(&self, va: usize) -> usize {
        va + (self.index + 1) * segment_layout::HART_SEGMENT_SIZE
    }

    pub fn hpa2gpa(&self, pa: usize) -> usize {
        pa - (self.index + 1) * segment_layout::HART_SEGMENT_SIZE
    }

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
        if let Some(shadow_pgt) = self.shadow_state.shadow_page_tables.find_shadow_page_table(self.shadow_state.get_satp()) {
            // 由于 GHA 与 GPA 是同等映射的，因此翻译成的物理地址可以直接当虚拟地址用
            let pte = shadow_pgt.translate(vpn);
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

    /// 映射 IOMMU 
    fn try_map_iommu(&self, guest_pgt: &PageTable, shadow_pgt: &mut PageTable) {
        // 映射 QEMU Virt
        for index in (0..QEMU_VIRT_SIZE).step_by(PAGE_SIZE) {
            let gvpn = VirtPageNum::from((QEMU_VIRT_START + index) >> 12);
            if let Some(gppte) = guest_pgt.translate_gvpn(gvpn, self.memory.page_table()) {
                let gppn = gppte.ppn();
                let hvpn = self.memory.translate(VirtPageNum::from(gppn.0)).unwrap().ppn();
                let hppn = KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(hvpn.0)).unwrap().ppn();
                shadow_pgt.try_map(gvpn, hppn, PTEFlags::R | PTEFlags::W | PTEFlags::U);
            }
        }
    }

    fn try_map_guest_area(&self, guest_pgt: &PageTable, shadow_pgt: &mut PageTable) {
        for gva in (GUEST_KERNEL_VIRT_START_1..GUEST_KERNEL_VIRT_END_1).step_by(PAGE_SIZE) {
            let gvpn = VirtPageNum::from(gva >> 12);
            let gppn = guest_pgt.translate_gvpn(gvpn, &self.memory.page_table());
            // 如果 guest ppn 存在且有效
            // TODO: 将影子页表的标志位设置为不可写，当 Guest OS 修改页表的时候
            if let Some(gppn) = gppn {
                if gppn.is_valid() {
                    let hppn = PhysPageNum::from(self.gpa2hpa(gppn.ppn().0 << 12) >> 12);
                    let mut pte_flags = PTEFlags::U;
                    if gppn.readable() {
                        pte_flags |= PTEFlags::R;
                    }
                    if gppn.writable() {
                        pte_flags |= PTEFlags::W;
                    }
                    if gppn.executable() {
                        pte_flags |= PTEFlags::X;
                    }
                    shadow_pgt.try_map(gvpn, hppn, pte_flags)
                }
            }
        }
    }

    fn try_map_user_area(&self, guest_pgt: &PageTable, shadow_pgt: &mut PageTable) {
        for gva in (0..0x800_0000).step_by(PAGE_SIZE) {
            let gvpn = VirtPageNum::from(gva >> 12);
            let gppn = guest_pgt.translate_gvpn(gvpn, &self.memory.page_table());
            // 如果 guest ppn 存在且有效
            // TODO: 将影子页表的标志位设置为不可写，当 Guest OS 修改页表的时候
            if let Some(gppn) = gppn {
                if gppn.is_valid() {
                    let gpa = gppn.ppn().0 << 12;
                    // let hva = self.translate_guest_paddr(gpa).unwrap();
                    // let hvpn = VirtPageNum::from(hva >> 12);
                    // let hppn = KERNEL_SPACE.exclusive_access().translate(hvpn).unwrap().ppn();
                    let hppn = PhysPageNum::from(self.gpa2hpa(gpa) >> 12);
                    let mut pte_flags = PTEFlags::U;
                    if gppn.readable() {
                        pte_flags |= PTEFlags::R;
                    }
                    if gppn.writable() {
                        pte_flags |= PTEFlags::W;
                    }
                    if gppn.executable() {
                        pte_flags |= PTEFlags::X;
                    }
                    shadow_pgt.try_map(gvpn, hppn, pte_flags)
                }
            }
        }
    }

    fn try_map_user_trampoline(&self, guest_pgt: &PageTable, shadow_pgt: &mut PageTable) {
        // 映射 guest 跳板页
        let guest_trampoline_gvpn = VirtPageNum::from(GUEST_TRAMPOLINE >> 12);
        if let Some(guest_trampoline_gpte) = guest_pgt.translate_gvpn(guest_trampoline_gvpn, &self.memory.page_table()) {
            let guest_trampoline_gppn = guest_trampoline_gpte.ppn();
            // let guest_trampoline_hvpn = VirtPageNum::from(
            //     self.memory.translate(VirtPageNum::from(guest_trampoline_gppn.0)).unwrap().ppn().0
            // );
            // let guest_trampoline_hppn = KERNEL_SPACE.exclusive_access().translate(guest_trampoline_hvpn).unwrap().ppn();
            let guest_trampoline_hppn = PhysPageNum::from(self.gpa2hpa(guest_trampoline_gppn.0 << 12) >> 12);
            shadow_pgt.try_map(guest_trampoline_gvpn, guest_trampoline_hppn, PTEFlags::R | PTEFlags::X | PTEFlags::U);
        }
    }

    fn try_map_user_trap_context(&self, guest_pgt: &PageTable, shadow_pgt: &mut PageTable) {
        let guest_trap_context_gvpn = VirtPageNum::from(GUEST_TRAP_CONTEXT >> 12);
        if let Some(guest_trap_context_gpte) = guest_pgt.translate_gvpn(guest_trap_context_gvpn, &self.memory.page_table()) {
            let guest_trap_context_gppn = guest_trap_context_gpte.ppn();
            // let guest_trampoline_hvpn = VirtPageNum::from(
            //     self.memory.translate(VirtPageNum::from(guest_trap_context_gppn.0)).unwrap().ppn().0
            // );
            // let guest_trap_context_hppn = KERNEL_SPACE.exclusive_access().translate(guest_trampoline_hvpn).unwrap().ppn();
            let guest_trap_context_hppn = PhysPageNum::from(self.gpa2hpa(guest_trap_context_gppn.0 << 12) >> 12);
            shadow_pgt.try_map(guest_trap_context_gvpn, guest_trap_context_hppn, PTEFlags::R | PTEFlags::X | PTEFlags::U);
        }
    }


    /// 同步影子页表
    pub fn sync_shadow_page_table(&mut self) {
        let satp = self.shadow_state.get_satp();
        let root_gpa = (satp << 12) & 0x7f_ffff_ffff;
        let root_hppn = PhysPageNum::from(self.gpa2hpa(root_gpa) >> 12);
        let guest_pgt = PageTable::from_ppn(root_hppn);
        let shadow_page_table = self.shadow_state.shadow_page_tables.find_shadow_page_table_mut(satp).unwrap();
    }
    
    /// 验证需要映射的内存是否为客户页表的页表项，若为页表项，则将
    /// 权限位设置为不可写，以便在 Guest OS 修改页表项时陷入 VMM
    pub fn is_guest_page_table(&self, mut vaddr: usize) -> bool {
        // 虚拟地址页对齐
        vaddr = (vaddr >> 12) << 12;
        let root = (self.shadow_state.get_satp() << 12) & 0x7f_ffff_ffff;
        let root_vpn = VirtPageNum::from(root >> 12);
        let mut queue = VecDeque::new();
        let mut buffer = Vec::new();
        queue.push_back(root_vpn); 
        for _ in 0..3 {
            while !queue.is_empty() {
                let vpn = queue.pop_front().unwrap();
                let ppn = PhysPageNum::from(self.gpa2hpa(vpn.0 << 12) >> 12);
                let ptes = ppn.get_pte_array();
                for pte in ptes {
                    if pte.ppn().0 == (vaddr >> 12){ return true }
                    if pte.is_valid() {
                        buffer.push(VirtPageNum::from(pte.ppn().0))
                    }
                } 
            }
            while !buffer.is_empty() {
                queue.push_back(buffer.pop().unwrap());
            }
        }
        false
    }


    /// 对于页表的 PTE 的标志位应当标志为只读，用来同步 Guest Page Table 和 Shadow Page Table
    pub fn map_page_table(&self, root_gpa: usize, shadow_pgt: &mut PageTable) {
        let root_gvpn = VirtPageNum::from(root_gpa >> 12);
        // 广度优先搜索遍历所有页表
        let mut queue = VecDeque::new();
        let mut buffer = Vec::new();
        queue.push_back(root_gvpn); 
        for _ in 0..3 {
            while !queue.is_empty() {
                let vpn = queue.pop_front().unwrap();
                let ppn = PhysPageNum::from(self.gpa2hpa(vpn.0 << 12) >> 12);
                shadow_pgt.map(vpn, ppn, PTEFlags::R | PTEFlags::U);
                let ptes = ppn.get_pte_array();
                for pte in ptes {
                    if pte.is_valid() {
                        // hdebug!("pte ppn: {:#x}", pte.ppn().0);
                        buffer.push(VirtPageNum::from(pte.ppn().0))
                    }
                } 
            }
            while !buffer.is_empty() {
                queue.push_back(buffer.pop().unwrap());
            }
        }
    }

    /// 根据 satp 构建影子页表
    /// 需要将 GVA -> HPA
    pub fn install_shadow_page_table(&mut self, satp: usize) {
        if self.shadow_state.shadow_page_tables.find_shadow_page_table(satp).is_none() {
            // 如果影子页表中没有发现，新建影子页表
            // 根据 satp 获取 guest kernel 根页表的物理地址
            let root_gpa = (satp << 12) & 0x7f_ffff_ffff;
            let root_hppn = PhysPageNum::from(self.gpa2hpa(root_gpa) >> 12);
            let guest_pgt = PageTable::from_ppn(root_hppn);
            // 翻译的时候不能直接翻译，因为此时取出的 pte 都是 Guest OS 的物理地址，需要将 pte 翻译成 hypervisor 的地址
            // 即将 guest virtual address -> host virtual address
            // 最终翻译的结果为 GPA (Guest Physical Address)
            // 构建影子页表
            let mut shadow_pgt = PageTable::new();
            // 映射客户页表
            self.map_page_table(root_gpa, &mut shadow_pgt);
            // 尝试映射内核地址空间
            self.try_map_guest_area(&guest_pgt, &mut shadow_pgt);
            // 尝试映射用户地址空间
            self.try_map_user_area(&guest_pgt, &mut shadow_pgt);
            // 映射 IOMMU 
            self.try_map_iommu(&guest_pgt, &mut shadow_pgt);
            // 尝试映射用户空间的跳板页
            self.try_map_user_trampoline(&guest_pgt, &mut shadow_pgt);
            // 尝试映射用户空间 Trap Context
            self.try_map_user_trap_context(&guest_pgt, &mut shadow_pgt);

            // 映射内核跳板页
            let trampoline_hppn = KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(TRAMPOLINE >> 12)).unwrap().ppn();
            shadow_pgt.map(VirtPageNum::from(TRAMPOLINE >> 12), trampoline_hppn, PTEFlags::R | PTEFlags::X);


            // 映射 TRAP CONTEXT(TRAP 实际上在 Guest OS 中并没有被映射，但是我们在切换跳板页的时候需要使用到)
            let trapctx_hvpn = VirtPageNum::from(self.translate_guest_paddr(TRAP_CONTEXT).unwrap() >> 12);
            let trapctx_hppn = KERNEL_SPACE.exclusive_access().translate(trapctx_hvpn).unwrap().ppn();
            shadow_pgt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);

            // 测试映射是否正确
            assert_eq!(shadow_pgt.translate(0x80000.into()).unwrap().readable(), true);
            assert_eq!(shadow_pgt.translate(0x80000.into()).unwrap().is_valid(), true);
            assert_eq!(shadow_pgt.translate(0x80329.into()).unwrap().readable(), true);
            assert_eq!(shadow_pgt.translate(0x80329.into()).unwrap().is_valid(), true);
            assert_eq!(shadow_pgt.translate(VirtPageNum(TRAMPOLINE >> 12)).unwrap().readable(), true);
            assert_eq!(shadow_pgt.translate(VirtPageNum(TRAP_CONTEXT >> 12)).unwrap().writable(), true);

            let shadow_page_table = ShadowPageTable::new( satp, shadow_pgt);
            // 修改影子页表
            // self.shadow_state.shadow_pgt.replace_guest_pgt(shadow_pgt);
            self.shadow_state.shadow_page_tables.push(shadow_page_table);
            hdebug!("Success to construct shadow page table......");
        }
    }

}