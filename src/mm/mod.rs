mod memory_set;
mod memory_region;

pub use memory_set::{ remap_test, guest_kernel_test };
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use memory_region::MemoryRegion;

use crate::page_table::{PageTableSv39};

pub fn vm_init(guest_kernel_memory: &MemorySet<PageTableSv39>) {
    KERNEL_SPACE.exclusive_access().hyper_load_guest_kernel(guest_kernel_memory);
    KERNEL_SPACE.exclusive_access().activate();
}