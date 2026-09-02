mod uart;
mod plic;
mod virtio;
pub use uart::Uart;
// PR #16 (fix-bug/modern-rust-toolchain): keep device types available to the
// API even when a particular guest configuration does not construct them.
#[allow(unused_imports)]
pub use plic::HostPlic;
#[allow(unused_imports)]
pub use virtio::{ VirtIO, is_device_access };


/// Software emulated device used in VMM
pub struct VirtDevice {
    pub qemu_virt_tester: qemu_virt::QemuVirtTester,
    /// PR fix-bug/virtio-dma-translation: per-guest VirtIO MMIO/DMA state.
    pub virtio: VirtIO,
    pub uart: Uart
}

impl VirtDevice {
    pub fn new(guest_id: usize) -> Self {
        Self { 
            qemu_virt_tester: qemu_virt::QemuVirtTester::new(),
            virtio: VirtIO::new(0x1000_1000),
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
