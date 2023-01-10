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

use core::borrow::BorrowMut;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, PageTableEntry};
use page_table::{PTEFlags, PageTable};

use crate::GUEST_KERNEL;

/// 将客户操作系统加载到对应的物理地址
pub unsafe fn load_guest_kernel(kernel_memory: &mut MemorySet, guest_kernel: &[u8]) -> usize {
    for i in 0..4 {
        print!("{:#x} ", &guest_kernel[i])
    }
    print!("\n");
    println!("Loading guest kernel......");
    let guest_kernel_len = guest_kernel.len();
    use crate::config::GUEST_KERNEL_PHY_START_1;
    // 将客户操作系统写入对应的物理地址
    println!("[hypervisor] guest kernel size: {:#x}", guest_kernel_len);
    let guest_kernel_data = core::slice::from_raw_parts_mut(GUEST_KERNEL_PHY_START_1 as *mut u8, guest_kernel_len);
    core::ptr::copy(guest_kernel.as_ptr(), guest_kernel_data.as_mut_ptr() , guest_kernel_len);
    let entry_point = kernel_memory.map_guest_kernel(&guest_kernel_data);
    return entry_point
}

/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    let mut kernel_memory_set = KERNEL_SPACE.exclusive_access();
    let entry_point = unsafe{ load_guest_kernel(&mut kernel_memory_set, &GUEST_KERNEL) };
    println!("[hypervisor] guest kernel entry point: {:#x}", entry_point);
    kernel_memory_set.activate();
}
