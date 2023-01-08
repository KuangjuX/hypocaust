//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`task`]: Task management
//! - [`syscall`]: System call handling and implementation
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`task::run_first_task()`] and for the first time go to
//! userspace.

// #![deny(missing_docs)]
// #![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]

use crate::mm::{MemorySet, KERNEL_SPACE};

extern crate alloc;

#[macro_use]
extern crate bitflags;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod config;
mod lang_items;
mod loader;
mod mm;
mod sbi;
mod sync;
pub mod syscall;
pub mod task;
mod timer;
pub mod trap;

core::arch::global_asm!(include_str!("entry.asm"));
// core::arch::global_asm!(include_str!("link_app.S"));

#[link_section = ".initrd"]
#[cfg(feature = "embed_guest_kernel")]
static GUEST_KERNEL: [u8;include_bytes!("../guest_kernel").len()] = 
 *include_bytes!("../guest_kernel");

 #[cfg(not(feature = "embed_guest_kernel"))]
 static GUEST_KERNEL: [u8; 0] = [];


/// clear BSS segment
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[no_mangle]
/// the rust entry-point of os
pub fn hentry() -> ! {
    clear_bss();
    println!("[hypervisor] Hello Hypocaust");
    // println!("[kernel] Guest Kernel len: {:#x}", GUEST_KERNEL.len());
    // println!("[kernel] guest kernel address: {:#x}", GUEST_KERNEL.as_ptr() as usize);
    // for i in 0..20 {
    //     print!("{:#x} ", &GUEST_KERNEL[i]);
    // }
    // 将客户操作系统记载入对应的物理地址
    let (guest_kernel_area, entry_point) = unsafe{ load_guest_kernel(&GUEST_KERNEL) };
    mm::init();
    // KERNEL_SPACE.exclusive_access().insert_framed_area(start_va, end_va, permission)
    mm::remap_test();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();

    loop{}
}

/// 将客户操作系统加载到对应的物理地址
pub unsafe fn load_guest_kernel(guest_kernel: &[u8]) -> (MemorySet, usize){
    println!("Loading guest kernel......");
    let guest_kernel_len = guest_kernel.len();
    use crate::config::{ GUEST_KERNEL_PHY_START, MAX_GUEST_KERNEL_PHY_END };
    let max_len = MAX_GUEST_KERNEL_PHY_END - GUEST_KERNEL_PHY_START;
    // 清空客户操作系统物理地址内存
    core::slice::from_raw_parts_mut(GUEST_KERNEL_PHY_START as *mut u8, max_len).fill(0);
    // 将客户操作系统写入对应的物理地址
    let guest_kernel_dst = core::slice::from_raw_parts_mut(GUEST_KERNEL_PHY_START as *mut u8, guest_kernel_len);
    guest_kernel_dst.copy_from_slice(guest_kernel);
    let (guest_kernel_area, entry_point) = MemorySet::map_guest_kernel(&guest_kernel_dst);
    (guest_kernel_area, entry_point)
}

