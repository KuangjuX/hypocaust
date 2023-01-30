//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod memory_set;
mod page_table;
mod memory_region;


pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange, PPNRange};
pub use memory_set::{ remap_test, guest_kernel_test };
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry};
pub use page_table::{PTEFlags, PageTable};
pub use memory_region::MemoryRegion;



pub fn vm_init(guest_kernel_memory: &MemorySet) {
    KERNEL_SPACE.exclusive_access().hyper_load_guest_kernel(guest_kernel_memory);
    KERNEL_SPACE.exclusive_access().activate();
}
