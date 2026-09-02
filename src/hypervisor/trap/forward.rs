use riscv::addr::BitField;
use riscv::register::{scause, stval};

use crate::constants::csr::status::{STATUS_SIE_BIT, STATUS_SPP_BIT};
use crate::page_table::PageTable;
use crate::debug::PageDebug;
use crate::guest::Vcpu;
use super::TrapContext;

/// 检测 Guest OS 是否发生中断，若有则进行转发
pub fn maybe_forward_interrupt<P: PageTable + PageDebug>(guest: &mut Vcpu<P>) {
    let guest_was_in_smode = guest.smode;
    let ctx: &mut TrapContext = guest.trap_cx_ppn.get_mut();
    let state = &mut guest.shadow_state;
    let Some(interrupt) = state.csrs.next_enabled_interrupt() else {
        return;
    };
    // PR #18 (fix-bug/smode-interrupt-forwarding): an S-mode interrupt is globally
    // enabled when the guest runs below S-mode, or when it runs in S-mode with
    // SIE set. Keep a masked pending interrupt queued for a later boundary.
    let globally_enabled = !guest_was_in_smode
        || state.csrs.sstatus.get_bit(STATUS_SIE_BIT);
    if globally_enabled {
        // PR #40 selects one cause with the RISC-V SEI > SSI > STI priority
        // order while leaving the level-sensitive pending bit asserted.
        state.csrs.scause = (1 << 63) | interrupt.cause();
        state.csrs.stval = 0;
        state.csrs.sepc = ctx.sepc;
        state.push_sie();
        // SPP records the interrupted virtual mode; execution then enters the
        // guest's S-mode trap vector.
        state.csrs.sstatus.set_bit(STATUS_SPP_BIT, guest_was_in_smode);
        guest.smode = true;
        ctx.sepc = state.csrs.stvec;
    }
}

/// 向 guest kernel 转发异常
pub fn forward_exception<P: PageTable + PageDebug>(guest: &mut Vcpu<P>, ctx: &mut TrapContext) {
    let guest_was_in_smode = guest.smode;
    let state = &mut guest.shadow_state;
    state.csrs.scause = scause::read().code();
    state.csrs.sepc = ctx.sepc;
    state.csrs.stval = stval::read();
    // PR #20 (fix-bug/exception-entry-interrupt-state): emulate the architectural
    // trap-entry SIE -> SPIE transition before guest S-mode code can be interrupted.
    state.push_sie();
    // PR #18 (fix-bug/smode-interrupt-forwarding): preserve the pre-trap virtual
    // mode in SPP and track trap-handler execution independently as S-mode.
    state.csrs.sstatus.set_bit(STATUS_SPP_BIT, guest_was_in_smode);
    guest.smode = true;
    ctx.sepc = state.csrs.stvec;
}
