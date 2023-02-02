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
mod fault;
mod page_fault;

use crate::constants::layout::{TRAMPOLINE, TRAP_CONTEXT};
use crate::guest::{current_user_token, current_trap_cx, GUEST_KERNEL_MANAGER};


use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec, sepc, sscratch
};
pub use context::TrapContext;
use self::fault::{ifault, timer_handler, maybe_forward_interrupt};
use self::page_fault::handle_page_fault;

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
    let ctx = current_trap_cx();
    let scause = scause::read();
    let stval = stval::read();
    // get guest kernel
    let mut inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let id = inner.run_id;
    let guest = &mut inner.kernels[id];
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ifault(guest, ctx);
        },
        Trap::Exception(Exception::Breakpoint) => { 
            ifault(guest, ctx);
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) 
        | Trap::Exception(Exception::InstructionPageFault)
        => {
            // hdebug!("scause: {:?}", scause.cause());
            // pfault(guest, ctx);
            if !handle_page_fault(guest, ctx) {
                panic!("forward exception");
                // forward_exception(guest, ctx);
            }
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            ifault(guest, ctx);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            timer_handler(guest);
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
    drop(inner);
    trap_return();
}

#[no_mangle]
/// set the new addr of __restore asm function in TRAMPOLINE page,
/// set the reg a0 = trap_cx_ptr, reg a1 = phy addr of usr page table,
/// finally, jump to new addr of __restore asm function
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let trap_cx_ptr = TRAP_CONTEXT;
    let user_satp = current_user_token();
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
    // print_hypervisor_backtrace(_trap_cx);
    let scause= scause::read();
    let sepc = sepc::read();
    match scause.cause() {
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::LoadFault) | Trap::Exception(Exception::LoadPageFault)=> {
            let stval = stval::read();
            panic!("scause: {:?}, sepc: {:#x}, stval: {:#x}", scause.cause(), _trap_cx.sepc, stval);
        },
        _ => { panic!("scause: {:?}, spec: {:#x}", scause.cause(), sepc)}
    }
}


