//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;



pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::{ remap_test, guest_kernel_test };
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry};
pub use page_table::{PTEFlags, PageTable};


// /// 将客户操作系统加载到对应的物理地址
// pub unsafe fn load_guest_kernel(kernel_memory: &mut MemorySet, guest_kernel: &[u8]) -> usize {
//     println!("Loading guest kernel......");
//     let entry_point = kernel_memory.map_guest_kernel(&guest_kernel);
//     return entry_point
// }

/// initiate heap allocator, frame allocator and kernel space
pub fn heap_init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
}

pub fn vm_init(guest_kernel_memory: &MemorySet) {
    KERNEL_SPACE.exclusive_access().hyper_load_guest_kernel(guest_kernel_memory);
    KERNEL_SPACE.exclusive_access().activate();
}
