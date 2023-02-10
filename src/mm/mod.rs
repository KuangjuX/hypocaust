mod memory_set;
mod memory_region;

pub use memory_set::{ remap_test, guest_kernel_test };
pub use memory_set::{MapPermission, MemorySet};
pub use memory_region::MemoryRegion;

use crate::hypervisor::HYPOCAUST;
use crate::page_table::PageTableSv39;

pub fn vm_init(guest_kernel_memory: &MemorySet<PageTableSv39>) {
    let hypocaust = HYPOCAUST.lock();
    let mut hyper_space = hypocaust.hyper_space.exclusive_access();
    hyper_space.hyper_load_guest_kernel(guest_kernel_memory);
    hyper_space.activate();
}