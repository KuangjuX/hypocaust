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
static GUEST_KERNEL: [u8;include_bytes!("../minikernel/target/riscv64gc-unknown-none-elf/debug/minikernel.bin").len()] = 
 *include_bytes!("../minikernel/target/riscv64gc-unknown-none-elf/debug/minikernel.bin");

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
    mm::init();
    mm::remap_test();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
    loop{}
}

const GUEST_KERNEL_BASE_ADDR: usize = 0x80400000;
const GUEST_KERNEL_SIZE_LIMIT: usize = 0x200000;

/// 加载 guest kernel 二进制文件
pub fn load_guest_kernel() {
    println!("Loading Guest Kernel......");
    println!("guest kernel address :{:#x}", GUEST_KERNEL.as_ptr() as usize);
    println!("guest kernel size: {:#x}", GUEST_KERNEL.len());
    // 清理空间
    unsafe{ core::slice::from_raw_parts_mut(GUEST_KERNEL_BASE_ADDR as *mut u8, GUEST_KERNEL_SIZE_LIMIT) };
    // 将 guest kernel 拷贝到对应的地址中
    unsafe{
        core::ptr::copy(
            GUEST_KERNEL.as_ptr(), 
            GUEST_KERNEL_BASE_ADDR as *mut u8, 
            GUEST_KERNEL.len()
        );
    }
}