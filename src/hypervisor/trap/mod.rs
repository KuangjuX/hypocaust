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
use crate::identity::HartId;

use core::arch::{asm, global_asm};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sie, stval, stvec, sepc, sscratch
};
pub use context::TrapContext;
use self::inst_fault::{ifault, decode_instruction_at_address};
use self::page_fault::handle_page_fault;
use self::device::{handle_device_mmio, handle_time_interrupt, poll_device_completions};
use self::forward::{forward_exception, maybe_forward_interrupt};


global_asm!(include_str!("trap.S"));

/// PR #48 (`fix-bug/guest-exception-forwarding`) separates synchronous Guest
/// exceptions from physical Host interrupts. An unsupported Guest exception
/// is architectural input for one vCPU, not a reason to panic the Hypervisor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TrapRoute {
    EmulateInstruction,
    HandlePageFault,
    ForwardGuestException,
    Preempt,
    HostSoftwareInterrupt,
    FatalHostInterrupt,
}

fn route_trap(trap: Trap) -> TrapRoute {
    match trap {
        Trap::Exception(Exception::UserEnvCall | Exception::IllegalInstruction) => {
            TrapRoute::EmulateInstruction
        }
        Trap::Exception(Exception::LoadPageFault | Exception::StorePageFault) => {
            TrapRoute::HandlePageFault
        }
        Trap::Exception(_) => TrapRoute::ForwardGuestException,
        Trap::Interrupt(Interrupt::SupervisorTimer) => TrapRoute::Preempt,
        Trap::Interrupt(Interrupt::SupervisorSoft) => TrapRoute::HostSoftwareInterrupt,
        Trap::Interrupt(_) => TrapRoute::FatalHostInterrupt,
    }
}

/// PR #48 exercises the exception/interrupt isolation decision without
/// requiring a Guest to deliberately crash during the QEMU boot regression.
pub(crate) fn exception_routing_self_test() {
    assert_eq!(
        route_trap(Trap::Exception(Exception::Breakpoint)),
        TrapRoute::ForwardGuestException,
    );
    assert_eq!(
        route_trap(Trap::Exception(Exception::IllegalInstruction)),
        TrapRoute::EmulateInstruction,
    );
    assert_eq!(
        route_trap(Trap::Interrupt(Interrupt::SupervisorTimer)),
        TrapRoute::Preempt,
    );
}



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

/// PR #40 enables Host scheduler/IPI events on every online hart, including
/// while that hart is executing Guest code.
pub fn enable_software_interrupt() {
    unsafe { asm!("csrsi sie, 2") };
}

pub fn disable_timer_interrupt() {
    unsafe{ sie::clear_stimer(); }
}


#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler() -> ! {
    set_kernel_trap_entry();
    let hart_id = HartId::current();
    let mut hypervisor_guard = HYPOCAUST.lock();
    let hypervisor = {&mut *hypervisor_guard}.as_mut().unwrap();
    let ctx = hypervisor.current_trap_cx(hart_id);
    let scause = scause::read();
    let stval = stval::read();
    // get guest kernel
    let (guest, device_bus) = hypervisor.current_vcpu_and_device_bus(hart_id);
    // PR #24 (`feature/shadow-paging-profile`) counts every transition from
    // the deprivileged guest into Hypocaust to correlate traps with paging work.
    guest.shadow_state.shadow_paging_stats.record_trap();
    let preempt = match route_trap(scause.cause()) {
        TrapRoute::EmulateInstruction => {
            ifault(guest, ctx);
            false
        },
        // PR #17 (fix-bug/virtio-dma-translation): VirtIO reads fault by
        // design and must enter the same MMIO emulator as register writes.
        TrapRoute::HandlePageFault => {
            if !handle_page_fault(guest, device_bus, ctx) {
                htracking!("forward page exception sepc -> {:#x}", ctx.sepc);
                forward_exception(guest, ctx);
            }
            false
        }
        TrapRoute::ForwardGuestException => {
            // PR #48 forwards breakpoints, access faults, misaligned accesses,
            // and other synchronous exceptions only into the current vCPU.
            forward_exception(guest, ctx);
            false
        }
        TrapRoute::Preempt => {
            handle_time_interrupt(guest);
            poll_device_completions(guest, device_bus);
            true
        },
        TrapRoute::HostSoftwareInterrupt => {
            // PR #40 uses this Host IPI to make the current vCPU re-arbitrate
            // its own virtual pending bits; it is not itself a Guest SSIP.
            unsafe { asm!("csrci sip, 2") };
            false
        },
        TrapRoute::FatalHostInterrupt => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x} spec: {:#x} smode -> {}!",
                scause.cause(),
                stval,
                ctx.sepc,
                guest.smode
            );
        }
    };
    if preempt {
        hypervisor.preempt(hart_id)
            .expect("preemption left Host hart without a runnable vCPU");
    }
    drop(hypervisor_guard);
    trap_return();
}

#[no_mangle]
/// set the new addr of __restore asm function in TRAMPOLINE page,
/// set the reg a0 = trap_cx_ptr, reg a1 = phy addr of usr page table,
/// finally, jump to new addr of __restore asm function
pub fn trap_return() -> ! {
    set_user_trap_entry();
    let hart_id = HartId::current();
    let trap_cx_ptr = TRAP_CONTEXT;
    let (user_satp, flush_asid) = {
        let mut hypervisor_guard = HYPOCAUST.lock();
        let hypervisor = hypervisor_guard.as_mut().unwrap();
        // PR #40 arbitrates pending interrupts for whichever vCPU the
        // scheduler selected, closing IPI-versus-preemption races.
        maybe_forward_interrupt(hypervisor.current_vcpu(hart_id));
        hypervisor.prepare_current_user_token(hart_id)
    };
    if let Some(asid) = flush_asid {
        // PR #26 (`feature/shadow-page-table-asid`) flushes only a dirty
        // destination; clean shadow ASIDs retain translations across traps.
        unsafe { asm!("sfence.vma x0, {asid}", asid = in(reg) asid) };
    }
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
pub fn trap_from_kernel(_trap_cx: &TrapContext) {
    let scause= scause::read();
    let sepc = sepc::read();
    match scause.cause() {
        Trap::Interrupt(Interrupt::SupervisorSoft) => {
            // PR #38 returns an idle scheduler hart from this trap so it can
            // check the run queue populated before the sender issued its IPI.
            unsafe { asm!("csrci sip, 2") };
        },
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::LoadFault) | Trap::Exception(Exception::LoadPageFault)=> {
            print_hypervisor_backtrace(_trap_cx);
            let stval = stval::read();
            panic!("scause: {:?}, sepc: {:#x}, stval: {:#x}", scause.cause(), _trap_cx.sepc, stval);
        },
        _ => {
            print_hypervisor_backtrace(_trap_cx);
            panic!("scause: {:?}, spec: {:#x}, stval: {:#x}", scause.cause(), sepc, stval::read())
        }
    }
}
