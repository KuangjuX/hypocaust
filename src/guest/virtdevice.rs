use crate::page_table::MemoryRegion;

/// Software emulated device used in VMM
pub struct VirtDevice {
    pub qemu_virt_tester: QemuVirtTester
}

impl VirtDevice {
    pub fn new() -> Self {
        Self { 
            qemu_virt_tester: QemuVirtTester::new()
        }
    }

}



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