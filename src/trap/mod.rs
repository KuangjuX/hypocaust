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

use crate::constants::layout::{TRAMPOLINE, TRAP_CONTEXT};
use crate::guest::{current_user_token, write_shadow_csr, current_trap_cx};
use crate::mm::PhysAddr;
// use crate::task::{
//     current_trap_cx, current_user_token, exit_current_and_run_next, suspend_current_and_run_next,
// };
use crate::timer::set_next_trigger;
use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec, sepc
};

global_asm!(include_str!("trap.S"));

/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    set_kernel_trap_entry();
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE as usize, TrapMode::Direct);
    }
}

/// enable timer interrupt in sie CSR
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let ctx = current_trap_cx();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // cx.sepc += 4;
            // cx.x[10] = syscall(cx.x[17], [cx.x[10], cx.x[11], cx.x[12]]) as usize;
            println!("[hypervisor] user env call");
            panic!()
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadFault)
        | Trap::Exception(Exception::LoadPageFault) => {
            println!("[hypervisor] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}.", stval, ctx.sepc);
            let epc = ctx.sepc;
            let i1 = unsafe{ core::ptr::read(epc as *const u16) };
            let len = riscv_decode::instruction_length(i1);
            let inst = match len {
                2 => i1 as u32,
                4 => unsafe{ core::ptr::read(epc as *const u32) },
                _ => unreachable!()
            };
            println!("[hypervisor] inst: {:#x}", inst);
            if let Ok(inst) = riscv_decode::decode(inst) {
                match inst {
                    riscv_decode::Instruction::Sd(i) => {
                        let rs1 = i.rs1();
                        let rs2 = i.rs2();
                        println!("[hypervisor] rs1: {}, rs2: {}", rs1, rs2);
                    },
                    _ => { panic!("[hypervisor] Unrecognized instruction") }
                }
            }else{
                println!("[hypervisr] Fail to parse instruction");
            }
            panic!()
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            // 读出发生异常的 guest kernel 物理地址(虚拟地址)
            // 目前没有影子页表，可以直接读取
            let epc = sepc::read();
            let i1 = unsafe{ core::ptr::read(epc as *const u16) };
            let len = riscv_decode::instruction_length(i1);
            let inst = match len {
                2 => i1 as u32,
                4 => unsafe{ core::ptr::read(epc as *const u32) },
                _ => unreachable!()
            };
            if let Ok(inst) = riscv_decode::decode(inst) {
                match inst {
                    riscv_decode::Instruction::Csrrc(i) => {

                    }
                    riscv_decode::Instruction::Csrrs(i) => {
                        
                    }
                    // 写 CSR 指令
                    riscv_decode::Instruction::Csrrw(i) => {
                        let csr = i.csr() as usize;
                        let rs = i.rs1() as usize;
                        println!("[hypervisor] csr: {}, rs: {}", csr, rs);
                        // 向 Shadow CSR 写入
                        let val = ctx.x[rs];
                        println!("[hypervisor] x{}: {:#x}", rs, val);
                        write_shadow_csr(csr, val);
                        // 更新地址
                        ctx.sepc += len;
                    },
                    _ => {
                        panic!("[hypervisor] Unrecognized instruction!");
                    }
                }
            }else{
                panic!("[hypervisor] Fail to decode expection instruction");
            }
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            // suspend_current_and_run_next();
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
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
    // println!("[hypervisor] user satp: {:#x}", user_satp);
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
/// Unimplement: traps/interrupts/exceptions from kernel mode
/// Todo: Chapter 9: I/O device
pub fn trap_from_kernel() -> ! {
    let scause= scause::read();
    match scause.cause() {
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::LoadFault) | Trap::Exception(Exception::LoadPageFault)=> {
            let stval = stval::read();
            let sepc = sepc::read();
            panic!("scause: {:?}, sepc: {:#x}, stval: {:#x}", scause.cause(), sepc, stval);
        },
        _ => { panic!("scause: {:?}", scause.cause())}
    }
}

pub use context::TrapContext;
