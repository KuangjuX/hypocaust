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

use riscv::register::{sstatus, sepc};
use config::GUEST_KERNEL_VIRT_START_1;


core::arch::global_asm!(include_str!("entry.asm"));

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
pub fn hentry(hart_id: usize, device_tree_blob: usize) -> ! {
    clear_bss();
    println!("[hypervisor] Hello Hypocaust");
    println!("[hypervisor] hart_id: {}, device tree blob: {:#x}", hart_id, device_tree_blob);
    mm::init();
    trap::init();
    mm::remap_test();
    // trap::enable_timer_interrupt();
    // timer::set_next_trigger();
    // 返回运行 guest kernel

    sepc::write(GUEST_KERNEL_VIRT_START_1);
    unsafe{
        sstatus::set_spp(sstatus::SPP::User);
        sstatus::set_sie();
    }
    unsafe{
        core::arch::asm!(
            "li ra, 0",
            "li sp, 0",
            "li gp, 0",
            "li tp, 0",
            "li t0, 0",
            "li t1, 0",
            "li t2, 0",
            "li s0, 0",
            "li s1, 0",
            "li a0, 0",
            "li a2, 0",
            "li a3, 0",
            "li a4, 0",
            "li a5, 0",
            "li a6, 0",
            "li a7, 0",
            "li s2, 0",
            "li s3, 0",
            "li s4, 0",
            "li s5, 0",
            "li s6, 0",
            "li s7, 0",
            "li s8, 0",
            "li s9, 0",
            "li s10, 0",
            "li s11, 0",
            "li t3, 0",
            "li t4, 0",
            "li t5, 0",
            "li t6, 0",
            "sret"
        );
    }
    loop{}
}



