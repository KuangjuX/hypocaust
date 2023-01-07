#![no_std]
#![no_main]
#![feature(panic_info_message)]

#[macro_use]
mod sbi;
mod lang_items;
mod console;
mod boards;
mod trap;
mod sync;

use core::arch::global_asm;

global_asm!(include_str!("asm/entry.asm"));


#[link_section = ".initrd"]
#[cfg(feature = "embed_guest_kernel")]
static GUEST_KERNEL: [u8;include_bytes!("../minikernel/target/riscv64gc-unknown-none-elf/debug/minikernel.bin").len()] = 
 *include_bytes!("../minikernel/target/riscv64gc-unknown-none-elf/debug/minikernel.bin");

 #[cfg(not(feature = "embed_guest_kernel"))]
 static GUEST_KERNEL: [u8; 0] = [];


/// clear BSS segment
pub fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

/// hypocaust 的入口地址，进入 S mode
#[no_mangle]
pub fn hentry() -> ! {
    clear_bss();
    println!("Hello, Hypocaust!");
    load_guest_kernel();
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

