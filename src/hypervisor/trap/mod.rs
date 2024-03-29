//! Trap handling functionality
//!
//! For rCore, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, and syscalls go
//! to [`syscall()`].
mod context;
mod inst_fault;
mod page_fault;
mod device;
mod forward;

use crate::constants::layout::{TRAMPOLINE, TRAP_CONTEXT};
use crate::debug::print_hypervisor_backtrace;
use crate::hypervisor::HYPOCAUST;

use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec, sepc, sscratch
};
pub use context::TrapContext;
use self::inst_fault::{ifault, decode_instruction_at_address};
use self::page_fault::handle_page_fault;
use self::device::{ handle_qemu_virt, handle_time_interrupt };
use self::forward::{forward_exception, maybe_forward_interrupt};


global_asm!(include_str!("trap.S"));



/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    extern "C" {
        fn __alltraps();
        fn __alltraps_k();
    }
    let __alltraps_k_va = __alltraps_k as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        stvec::write(__alltraps_k_va, TrapMode::Direct);
        sscratch::write(trap_from_kernel as usize);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

/// enable timer interrupt in sie CSR
pub fn enable_timer_interrupt() {
    unsafe { sie::set_stimer(); }
}

pub fn disable_timer_interrupt() {
    unsafe{ sie::clear_stimer(); }
}


#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    unsafe{ HYPOCAUST.force_unlock(); }
    let mut hypervisor = HYPOCAUST.lock();
    let hypervisor = {&mut *hypervisor}.as_mut().unwrap();
    let ctx = hypervisor.current_trap_cx();
    let scause = scause::read();
    let stval = stval::read();
    // get guest kernel
    let guest = hypervisor.current_guest();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ifault(guest, ctx);
        },
        Trap::Exception(Exception::Breakpoint) => { 
            ifault(guest, ctx);
        }
        Trap::Exception(Exception::StorePageFault) => {
            if !handle_page_fault(guest, ctx) {
                htracking!("forward page exception sepc -> {:#x}", ctx.sepc);
                forward_exception(guest, ctx);
            }
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            ifault(guest, ctx);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            handle_time_interrupt(guest);
            // 可能转发中断
            maybe_forward_interrupt(guest, ctx);
        },
        _ => {  
            panic!(
                "Unsupported trap {:?}, stval = {:#x} spec: {:#x} smode -> {}!",
                scause.cause(),
                stval,
                ctx.sepc,
                guest.shadow_state.smode()
            );
        }
    }
    drop(hypervisor);
    trap_return();
}

#[no_mangle]
/// set the new addr of __restore asm function in TRAMPOLINE page,
/// set the reg a0 = trap_cx_ptr, reg a1 = phy addr of usr page table,
/// finally, jump to new addr of __restore asm function
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    unsafe{ HYPOCAUST.force_unlock(); }
    let hypervisor = HYPOCAUST.lock();
    let hypervisor = {&*hypervisor}.as_ref().unwrap();
    let user_satp = hypervisor.current_user_token();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",             // jump to new addr of __restore asm function
            restore_va = in(reg) restore_va,
            in("a0") trap_cx_ptr,      // a0 = virt addr of Trap Context
            in("a1") user_satp,        // a1 = phy addr of usr page table
            options(noreturn)
        );
    }
}

#[no_mangle]
pub fn trap_from_kernel(_trap_cx: &TrapContext) -> ! {
    print_hypervisor_backtrace(_trap_cx);
    let scause= scause::read();
    let sepc = sepc::read();
    match scause.cause() {
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::LoadFault) | Trap::Exception(Exception::LoadPageFault)=> {
            let stval = stval::read();
            panic!("scause: {:?}, sepc: {:#x}, stval: {:#x}", scause.cause(), _trap_cx.sepc, stval);
        },
        _ => { panic!("scause: {:?}, spec: {:#x}, stval: {:#x}", scause.cause(), sepc, stval::read())}
    }
}


