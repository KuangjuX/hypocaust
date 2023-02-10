use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use virtio_drivers::transport::Transport;
use virtio_drivers::transport::mmio::MmioTransport;


use crate::sync::UPSafeCell;
use crate::mm::MemorySet;
use crate::page_table::{PageTable, PageTableSv39};
use crate::debug::PageDebug;
use self::device::VirtIOBlock;
pub use self::hyp_alloc::FrameTracker;

use lazy_static::lazy_static;


pub mod device;
pub mod hyp_alloc;
pub mod trap;

pub struct Hypervisor<P: PageTable + PageDebug, T: Transport> {
    pub hyper_space: Arc<UPSafeCell<MemorySet<P>>>,
    pub frame_queue: Mutex<Vec<FrameTracker>>,
    pub virtio_blk: Option<VirtIOBlock<T>>
}

lazy_static! {
    pub static ref HYPOCAUST: Mutex<Hypervisor<PageTableSv39, MmioTransport>> = Mutex::new(
        Hypervisor { 
            hyper_space: Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) }),
            frame_queue: Mutex::new(Vec::new()),
            virtio_blk: None
        }
    );
}

impl<P: PageTable + PageDebug, T: Transport> Hypervisor<P, T> {
    pub fn add_virtio_blk(&mut self, virtio_blk: VirtIOBlock<T>) {
        self.virtio_blk.replace(virtio_blk);
    }
}
