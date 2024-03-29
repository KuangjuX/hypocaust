//! The main module and entrypoint
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(core_intrinsics)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)] 
#![deny(warnings)]
#![feature(naked_functions)]
#![feature(asm_const)]


extern crate alloc;

#[macro_use]
extern crate bitflags;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod constants;
mod lang_items;
mod page_table;
mod sbi;
mod sync;
mod timer;
mod guest;
mod debug;
mod mm;
mod device_emu;
mod hypervisor;



use crate::constants::layout::PAGE_SIZE;
use crate::guest::GuestKernel;
use crate::hypervisor::HYPOCAUST;
use crate::mm::MemorySet;

// use fdt::Fdt;

#[link_section = ".initrd"]
#[cfg(feature = "embed_guest_kernel")]
static GUEST_KERNEL: [u8;include_bytes!("../guest_kernel").len()] = 
 *include_bytes!("../guest_kernel");

 #[cfg(not(feature = "embed_guest_kernel"))]
 static GUEST_KERNEL: [u8; 0] = [];

 const BOOT_STACK_SIZE: usize = 16 * PAGE_SIZE;

#[link_section = ".bss.stack"]
/// hypocaust boot stack
static BOOT_STACK: [u8; BOOT_STACK_SIZE] = [0u8; BOOT_STACK_SIZE];

#[link_section = ".text.entry"]
#[export_name = "_start"]
#[naked]
/// hypocaust entrypoint
pub unsafe extern "C" fn start() -> ! {
    core::arch::asm!(
        // prepare stack
        "la sp, {boot_stack}",
        "li t2, {boot_stack_size}",
        "addi t3, a0, 1",
        "mul t2, t2, t3",
        "add sp, sp, t2",
        // enter hentry
        "call hentry",
        boot_stack = sym BOOT_STACK,
        boot_stack_size = const BOOT_STACK_SIZE,
        options(noreturn)
    )
}

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
        hdebug!("Hello Hypocaust");
        hdebug!("hart_id: {}, device tree blob: {:#x}", hart_id, device_tree_blob);
        let meta = hypervisor::fdt::MachineMeta::parse(device_tree_blob);
        // 初始化堆及帧分配器
        hypervisor::hyp_alloc::heap_init();
        hypervisor::initialize_vmm(meta);
        let mut hypervisor = HYPOCAUST.lock();
        let hypervisor = {&mut *hypervisor}.as_mut().unwrap();
        let guest_kernel_memory = MemorySet::new_guest_kernel(&GUEST_KERNEL);
        // 初始化虚拟内存
        mm::vm_init(&guest_kernel_memory);
        hypervisor::trap::init();
        // 测试重映射
        mm::remap_test();
        // 测试 guest kernel 内存映射
        mm::guest_kernel_test();
        // 开启时钟中断
        hypervisor::trap::enable_timer_interrupt();
        timer::set_default_next_trigger();
        // 创建用户态的 guest kernel 内存空间
        let user_guest_kernel_memory = MemorySet::create_user_guest_kernel(&guest_kernel_memory);
        let guest = GuestKernel::new(user_guest_kernel_memory, 0);
        // 开始运行 guest kernel
        hypervisor.add_guest(guest);
        hypervisor.run_guest(0)
    }else{
        unreachable!()
    }
}

