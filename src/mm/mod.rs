mod memory_set;
mod memory_region;

pub use memory_set::{guest_kernel_test, map_host_mmio, remap_test};
pub use memory_set::{LoadedGuestKernel, MapPermission, MemorySet};
pub use memory_region::MemoryRegion;

use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::page_table::PageTableSv39;

pub fn vm_init(guest_kernel_memory: &MemorySet<PageTableSv39>) {
    // PR #44 (`fix-bug/map-guest-ram-before-load`) maps and activates the VM's
    // complete RAM slot before `load_guest_kernel` copies the payload. Keep this
    // compatibility entry point, but do not remap the same Host pages here.
    let _ = guest_kernel_memory;
    HYPERVISOR_MEMORY.exclusive_access().activate();
}
