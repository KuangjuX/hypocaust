use core::ptr::NonNull;

use spin::Mutex;
use virtio_drivers::{Hal, transport::Transport, device::blk::VirtIOBlk};

use crate::hyp_alloc::{frame_alloc, frame_dealloc};
use crate::page_table::PhysPageNum;

use super::BlockDevice;

pub struct VirtIOBlock<T: Transport>(Mutex<VirtIOBlk<VirtioHal, T>>);

// pub fn virtio_blk<T: Transport>(transport: T) {
//     let mut blk = VirtIOBlk::<VirtioHal, T>::new(transport).expect("failed to create blk driver");

// }

impl<T: Transport + 'static> BlockDevice for VirtIOBlock<T> {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        self.0
            .lock()
            .read_block(block_id, buf)
            .expect("Error when reading VirtIOBlk");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.0
            .lock()
            .write_block(block_id, buf)
            .expect("Error when writing VirtIOBlk");
    }
}

pub struct VirtioHal;

unsafe impl Hal for VirtioHal {
    /// Allocates the given number of continguous physical pages of DMA memory for VirtIO use.
    /// 
    /// Returns both the physical address which the device can use to access the memory, and a pointer to the start of it 
    /// which the driver can use to access it.
    fn dma_alloc(pages: usize, _direction: virtio_drivers::BufferDirection) -> (virtio_drivers::PhysAddr, core::ptr::NonNull<u8>) {
        let mut ppn_base = PhysPageNum(0);
        for i in 0..pages {
            let frame = frame_alloc().unwrap();
            if i == 0 {
                ppn_base = frame.ppn;
            }
            assert_eq!(frame.ppn.0, ppn_base.0 + i);
        }
        let pa: virtio_drivers::PhysAddr  = ppn_base.0 << 12;
        let va = NonNull::new(pa as _).unwrap();
        (pa, va)
    }

    /// Reallocates the given contiguou physical DMA memory pages
    unsafe fn dma_dealloc(paddr: virtio_drivers::PhysAddr, _vaddr: core::ptr::NonNull<u8>, pages: usize) -> i32 {
        let mut ppn_base = PhysPageNum::from(paddr >> 12);
        for _ in 0..pages {
            frame_dealloc(ppn_base);
            ppn_base.0 += 1;
        }
        0
    }

    /// Converts a physical address used for MMIO to a virtual address which the driver can access
    /// 
    /// This is only used for MMIO addressed within BARs read from the device, for the PCI transport. It may check
    /// that the address range up to the given size is within the region expected for MMIO.
    unsafe fn mmio_phys_to_virt(paddr: virtio_drivers::PhysAddr, _size: usize) -> core::ptr::NonNull<u8> {
        NonNull::new(paddr as _).unwrap()
    }

    /// Shares the given memory range with the device, and return the physical address that the device can use to access it.
    /// 
    /// This may involve mapping the buffer into an IOMMU, giving the host permission to access the memory, or copying
    /// it to a special region where it canbe accessed.
    unsafe fn share(_buffer: core::ptr::NonNull<[u8]>, _direction: virtio_drivers::BufferDirection) -> virtio_drivers::PhysAddr {
        unimplemented!()
    }

    /// Unshares the given memory range from the device and (if necessary) copies it back to the original buffer.
    unsafe fn unshare(_paddr: virtio_drivers::PhysAddr, _buffer: core::ptr::NonNull<[u8]>, _direction: virtio_drivers::BufferDirection) {
        unimplemented!()
    }
}


unsafe impl<T: Transport> Sync for VirtIOBlock<T>{}
unsafe impl<T: Transport> Send for VirtIOBlock<T>{}