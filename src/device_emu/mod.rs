mod uart;
pub use uart::Uart;


/// Software emulated device used in VMM
pub struct VirtDevice {
    pub qemu_virt_tester: qemu_virt::QemuVirtTester,
    pub uart: Uart
}

impl VirtDevice {
    pub fn new(guest_id: usize) -> Self {
        Self { 
            qemu_virt_tester: qemu_virt::QemuVirtTester::new(),
            uart: Uart::new(guest_id)
        }
    }

}



mod qemu_virt {
    use crate::mm::MemoryRegion;
    /// Software emulated qemu virt test
    pub struct QemuVirtTester {
        pub mmregs: MemoryRegion<u32>
    }

    impl QemuVirtTester {
        pub fn new() -> Self {
            Self { 
                mmregs: MemoryRegion::new(0x10_0000, 0x1000)
            }
        }

        pub fn in_region(&self, addr: usize) -> bool {
            self.mmregs.in_region(addr)
        }

        pub fn base(&self) -> usize {
            self.mmregs.base()
        }
    }
}