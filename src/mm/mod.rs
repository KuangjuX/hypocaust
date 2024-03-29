mod memory_set;
mod memory_region;

pub use memory_set::{ remap_test, guest_kernel_test };
pub use memory_set::{MapPermission, MemorySet};
pub use memory_region::MemoryRegion;

use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::page_table::PageTableSv39;

pub fn vm_init(guest_kernel_memory: &MemorySet<PageTableSv39>) {
    let mut hypervisor_memory = HYPERVISOR_MEMORY.exclusive_access();
    hypervisor_memory.hyper_load_guest_kernel(guest_kernel_memory);
    hypervisor_memory.activate();
}