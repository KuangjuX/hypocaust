//! The main module and entrypoint

// #![deny(missing_docs)]
// #![deny(warnings)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(core_intrinsics)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)] 
#![deny(warnings)]


extern crate alloc;

#[macro_use]
extern crate bitflags;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod constants;
mod lang_items;
mod mm;
mod sbi;
mod sync;
mod timer;
pub mod trap;
mod guest;


use crate::{mm::MemorySet, guest::{GuestKernel, GUEST_KERNEL_MANAGER, run_guest_kernel}};



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
pub fn hentry(hart_id: usize, device_tree_blob: usize) -> ! {
    if hart_id == 0{
        clear_bss();
        println!("[hypervisor] Hello Hypocaust");
        println!("[hypervisor] hart_id: {}, device tree blob: {:#x}", hart_id, device_tree_blob);
        // 初始化堆及帧分配器
        mm::heap_init();
        let guest_kernel_memory = MemorySet::new_guest_kernel(&GUEST_KERNEL);
        // 初始化虚拟内存
        mm::vm_init(&guest_kernel_memory);
        trap::init();
        mm::remap_test();
        mm::guest_kernel_test();
        // 开启时钟中断
        // trap::enable_timer_interrupt();
        // timer::set_default_next_trigger();
        // 创建用户态的 guest kernel 内存空间
        let user_guest_kernel_memory = MemorySet::create_user_guest_kernel(&guest_kernel_memory);
        let guest_kernel = GuestKernel::new(user_guest_kernel_memory, 0);
        GUEST_KERNEL_MANAGER.push(guest_kernel);
        // 开始运行 guest kernel
        run_guest_kernel();
    }else{
        unreachable!()
    }
}



