use crate::mm::{PageTable, KERNEL_SPACE, VirtPageNum, PTEFlags, PhysAddr, VirtAddr, PhysPageNum};
use crate::constants::layout::{ PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, GUEST_KERNEL_VIRT_START_1, GUEST_KERNEL_VIRT_END_1 };
use crate::board::{ QEMU_VIRT_START, QEMU_VIRT_SIZE };
use super::GuestKernel;

impl GuestKernel {
        /// GPA -> HPA
    pub fn translate_guest_paddr(&self, paddr: usize) -> usize {
        let offset = paddr & 0xfff;
        let vpn: VirtPageNum = VirtAddr(paddr).floor();
        let ppn = self.memory.translate(vpn).unwrap().ppn();
        let vaddr: PhysAddr = ppn.into();
        let vaddr: usize = vaddr.into();
        vaddr + offset
    }
    /// 映射 IOMMU 
    pub fn iommu_map(&self, guest_pgt: &PageTable, shadow_pgt: &mut PageTable) {
        // 映射 QEMU Virt
        for index in (0..QEMU_VIRT_SIZE).step_by(PAGE_SIZE) {
            let gvpn = VirtPageNum::from((QEMU_VIRT_START + index) >> 12);
            let gppn = guest_pgt.translate_gvpn(gvpn, self.memory.page_table()).unwrap().ppn();
            let hvpn = self.memory.translate(VirtPageNum::from(gppn.0)).unwrap().ppn();
            let hppn = KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(hvpn.0)).unwrap().ppn();
            shadow_pgt.map(VirtPageNum::from(gvpn), hppn, PTEFlags::R | PTEFlags::W | PTEFlags::U);
        }
    }

    /// 根据 satp 构建影子页表
    /// 需要将 GVA -> HPA
    pub fn new_shadow_pgt(&mut self, satp: usize) {
        // 根据 satp 获取 guest kernel 根页表的物理地址
        let root_gpa = (satp << 12) & 0x7f_ffff_ffff;
        let root_hva = self.translate_guest_paddr(root_gpa);
        let root_hppn =  KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(root_hva >> 12)).unwrap().ppn();
        let guest_pgt = PageTable::from_ppn(root_hppn);
        // 翻译的时候不能直接翻译，而要加一个偏移量，即将 guest virtual address -> host virtual address
        // 最终翻译的结果为 GPA (Guest Physical Address)
        // 构建影子页表
        let mut shadow_pgt = PageTable::new();
        // 将根目录页表中的所有映射转成影子页表
        for gva in (GUEST_KERNEL_VIRT_START_1..GUEST_KERNEL_VIRT_END_1).step_by(PAGE_SIZE) {
            let gvpn = VirtPageNum::from(gva >> 12);
            // let gppn = guest.memory.translate(gvpn);
            let gppn = guest_pgt.translate_gvpn(gvpn, &self.memory.page_table());
            // 如果 guest ppn 存在且有效
            if let Some(gppn) = gppn {
                if gppn.is_valid() {
                    let gpa = gppn.ppn().0 << 12;
                    let hva = self.translate_guest_paddr(gpa);
                    let hvpn = VirtPageNum::from(hva >> 12);
                    let hppn = KERNEL_SPACE.exclusive_access().translate(hvpn).unwrap().ppn();
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
                    shadow_pgt.map(gvpn, hppn, pte_flags)
                }
            }
        }
        // 映射 IOMMU 
        self.iommu_map(&guest_pgt, &mut shadow_pgt);
        // 映射跳板页
        let trampoline_gppn = guest_pgt.translate_gvpn(VirtPageNum::from(TRAMPOLINE >> 12), &self.memory.page_table()).unwrap().ppn();
        let trampoline_hppn = KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(TRAMPOLINE >> 12)).unwrap().ppn();
        shadow_pgt.map(VirtPageNum::from(TRAMPOLINE >> 12), trampoline_hppn, PTEFlags::R | PTEFlags::X);
        hdebug!("trampoline gppn: {:?}, trampoline hppn: {:?}", trampoline_gppn, trampoline_hppn);

        // 映射 TRAP CONTEXT(TRAP 实际上在 Guest OS 中并没有被映射，但是我们在切换跳板页的时候需要使用到)
        let trapctx_hvpn = VirtPageNum::from(self.translate_guest_paddr(TRAP_CONTEXT) >> 12);
        let trapctx_hppn = KERNEL_SPACE.exclusive_access().translate(trapctx_hvpn).unwrap().ppn();
        shadow_pgt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);
        hdebug!("trap ctx hvpn: {:?}, trap ctx hppn: {:?}", trapctx_hvpn, trapctx_hppn);

        // 测试映射是否正确
        assert_eq!(shadow_pgt.translate(0x80000.into()).unwrap().readable(), true);
        assert_eq!(shadow_pgt.translate(0x80000.into()).unwrap().is_valid(), true);
        assert_eq!(shadow_pgt.translate(0x80329.into()).unwrap().readable(), true);
        assert_eq!(shadow_pgt.translate(0x80329.into()).unwrap().is_valid(), true);
        assert_eq!(shadow_pgt.translate(VirtPageNum(TRAMPOLINE >> 12)).unwrap().readable(), true);
        assert_eq!(shadow_pgt.translate(VirtPageNum(TRAP_CONTEXT >> 12)).unwrap().writable(), true);

        // 修改影子页表
        self.shadow_state.shadow_pgt.replace_guest_pgt(shadow_pgt);
        hdebug!("Success to construct shadow page table......");
    }
}