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
pub use self::fdt::MachineMeta;



pub mod device;
pub mod hyp_alloc;
pub mod trap;
pub mod fdt;

pub struct Hypervisor<P: PageTable + PageDebug, T: Transport> {
    pub hyper_space: Arc<UPSafeCell<MemorySet<P>>>,
    pub frame_queue: UPSafeCell<Vec<FrameTracker>>,
    pub virtio_blk: Option<VirtIOBlock<T>>,
    pub meta: MachineMeta
}


pub static HYPOCAUST: Mutex<Option<Hypervisor<PageTableSv39, MmioTransport>>> = Mutex::new(None);



pub fn initialize_vmm(meta: MachineMeta) {
    unsafe{ HYPOCAUST.force_unlock(); }
    let old = HYPOCAUST.lock().replace(
        Hypervisor{
            hyper_space: Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) }),
            frame_queue: unsafe{ UPSafeCell::new(Vec::new()) },
            virtio_blk: None,
            meta
        }
    );
    core::mem::forget(old);
}

pub fn add_virtio_blk(virtio_blk: VirtIOBlock<MmioTransport>) {
    let mut hypocaust = HYPOCAUST.lock();
    let hypocaust = (&mut *hypocaust).as_mut().unwrap();
    let old = hypocaust.virtio_blk.replace(virtio_blk);
    core::mem::forget(old);
}
