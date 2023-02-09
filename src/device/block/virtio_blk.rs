use virtio_drivers::Hal;
pub struct HalImpl;

unsafe impl Hal for HalImpl {
    #[allow(unused)]
    fn dma_alloc(pages: usize, direction: virtio_drivers::BufferDirection) -> (virtio_drivers::PhysAddr, core::ptr::NonNull<u8>) {
        unimplemented!()
    }

    #[allow(unused)]
    unsafe fn dma_dealloc(paddr: virtio_drivers::PhysAddr, vaddr: core::ptr::NonNull<u8>, pages: usize) -> i32 {
        unimplemented!()
    }

    #[allow(unused)]
    unsafe fn mmio_phys_to_virt(paddr: virtio_drivers::PhysAddr, size: usize) -> core::ptr::NonNull<u8> {
        unimplemented!()
    }

    #[allow(unused)]
    unsafe fn share(buffer: core::ptr::NonNull<[u8]>, direction: virtio_drivers::BufferDirection) -> virtio_drivers::PhysAddr {
        unimplemented!()
    }

    #[allow(unused)]
    unsafe fn unshare(paddr: virtio_drivers::PhysAddr, buffer: core::ptr::NonNull<[u8]>, direction: virtio_drivers::BufferDirection) {
        unimplemented!()
    }
}