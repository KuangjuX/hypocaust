use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use crate::hyp_alloc::FrameTracker;
use crate::sync::UPSafeCell;
use crate::mm::MemorySet;
use crate::page_table::{PageTable, PageTableSv39};
use crate::debug::PageDebug;

use lazy_static::lazy_static;


mod mm;

pub struct Hypervisor<P: PageTable + PageDebug> {
    pub hyper_space: Arc<UPSafeCell<MemorySet<P>>>,
    pub frame_queue: Vec<FrameTracker>
}

lazy_static! {
    /// a memory set instance through lazy_static! managing kernel space
    pub static ref HYPOCAUST: Mutex<Hypervisor<PageTableSv39>> = Mutex::new(
        Hypervisor { 
            hyper_space: Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) }),
            frame_queue: Vec::new()
        }
    );
}
