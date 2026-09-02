mod memory_set;
mod memory_region;

pub use memory_set::{ remap_test, guest_kernel_test };
pub use memory_set::{MapPermission, MemorySet};
pub use memory_region::MemoryRegion;

use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::page_table::PageTableSv39;

pub fn vm_init(guest_kernel_memory: &MemorySet<PageTableSv39>) {
    // PR #44 (`fix-bug/map-guest-ram-before-load`) maps and activates the VM's
    // complete RAM slot before `new_guest_kernel` copies the ELF. Keep this
    // compatibility entry point, but do not remap the same Host pages here.
    let _ = guest_kernel_memory;
    HYPERVISOR_MEMORY.exclusive_access().activate();
}
