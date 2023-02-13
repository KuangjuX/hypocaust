use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::mm::MemorySet;
use crate::page_table::PageTableSv39;
use crate::sync::UPSafeCell;

lazy_static! {
    pub static ref HYPERVISOR_MEMORY: Arc<UPSafeCell<MemorySet<PageTableSv39>>> = Arc::new(unsafe{ UPSafeCell::new(MemorySet::<PageTableSv39>::new_kernel())});
}