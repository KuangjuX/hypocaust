use riscv::addr::BitField;
use riscv::register::{scause, stval};

use crate::constants::csr::sie::SSIE_BIT;
use crate::constants::csr::sip::{SEIP_BIT, STIP_BIT};
use crate::constants::csr::status::{STATUS_SIE_BIT, STATUS_SPP_BIT};
use crate::page_table::PageTable;
use crate::debug::PageDebug;
use crate::guest::Vcpu;
use super::TrapContext;

/// 检测 Guest OS 是否发生中断，若有则进行转发
pub fn maybe_forward_interrupt<P: PageTable + PageDebug>(guest: &mut Vcpu<P>, ctx: &mut TrapContext) {
    // 没有发生中断，返回
    if !guest.shadow_state.interrupt { return }
    let guest_was_in_smode = guest.smode;
    let state = &mut guest.shadow_state;
    let pending = state.csrs.sie & state.csrs.sip;
    // PR #18 (fix-bug/smode-interrupt-forwarding): an S-mode interrupt is globally
    // enabled when the guest runs below S-mode, or when it runs in S-mode with
    // SIE set. Keep a masked pending interrupt queued for a later boundary.
    let globally_enabled = !guest_was_in_smode
        || state.csrs.sstatus.get_bit(STATUS_SIE_BIT);
    if globally_enabled && pending != 0 {
        // hdebug!("forward timer interrupt: sepc -> {:#x}", ctx.sepc);
        let cause = if state.csrs.sip.get_bit(SEIP_BIT) { 9 }
        else if state.csrs.sip.get_bit(STIP_BIT) { 5 }
        else if state.csrs.sip.get_bit(SSIE_BIT) { 1 }
        else{ unreachable!() };

        state.csrs.scause = (1 << 63) | cause;
        state.csrs.stval = 0;
        state.csrs.sepc = ctx.sepc;
        state.push_sie();
        // SPP records the interrupted virtual mode; execution then enters the
        // guest's S-mode trap vector.
        state.csrs.sstatus.set_bit(STATUS_SPP_BIT, guest_was_in_smode);
        guest.smode = true;
        ctx.sepc = state.csrs.stvec;
    }else if pending == 0 {
        state.interrupt = false;
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
