#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![deny(warnings)]

#[macro_use]
mod sbi;
mod lang_items;
mod console;
mod boards;

use core::arch::global_asm;

global_asm!(include_str!("asm/entry.asm"));




/// clear BSS segment
pub fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}


#[no_mangle]
pub fn rust_main() -> ! {
    clear_bss();
    println!("Hello, Guest Kernel!");
    loop{}
}

