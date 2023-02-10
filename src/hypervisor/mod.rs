use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;


use crate::sync::UPSafeCell;
use crate::mm::MemorySet;
use crate::page_table::{PageTable, PageTableSv39};
use crate::debug::PageDebug;
pub use self::hyp_alloc::FrameTracker;

use lazy_static::lazy_static;


pub mod device;
pub mod hyp_alloc;
pub mod trap;

pub struct Hypervisor<P: PageTable + PageDebug> {
    pub hyper_space: Arc<UPSafeCell<MemorySet<P>>>,
    pub frame_queue: Mutex<Vec<FrameTracker>>
}

lazy_static! {
    /// a memory set instance through lazy_static! managing kernel space
    pub static ref HYPOCAUST: Mutex<Hypervisor<PageTableSv39>> = Mutex::new(
        Hypervisor { 
            hyper_space: Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) }),
            frame_queue: Mutex::new(Vec::new())
        }
    );
}
