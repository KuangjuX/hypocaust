use riscv::addr::BitField;
use riscv::register::{scause, stval};

use crate::constants::csr::sie::SSIE_BIT;
use crate::constants::csr::sip::{SEIP_BIT, STIP_BIT};
use crate::constants::csr::status::{STATUS_SIE_BIT, STATUS_SPP_BIT};
use crate::page_table::PageTable;
use crate::debug::PageDebug;
use crate::guest::GuestKernel;
use super::TrapContext;

/// 检测 Guest OS 是否发生中断，若有则进行转发
pub fn maybe_forward_interrupt<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>, ctx: &mut TrapContext) {
    // 没有发生中断，返回
    if !guest.shadow_state.interrupt { return }
    let state = &mut guest.shadow_state;
    // 当前状态处于用户态，且开启中断并有中断正在等待
    if (!state.smode() && state.csrs.sstatus.get_bit(STATUS_SIE_BIT)) && (state.csrs.sie & state.csrs.sip != 0) {
        // hdebug!("forward timer interrupt: sepc -> {:#x}", ctx.sepc);
        let cause = if state.csrs.sip.get_bit(SEIP_BIT) { 9 }
        else if state.csrs.sip.get_bit(STIP_BIT) { 5 }
        else if state.csrs.sip.get_bit(SSIE_BIT) { 1 }
        else{ unreachable!() };

        state.csrs.scause = (1 << 63) | cause;
        state.csrs.stval = 0;
        state.csrs.sepc = ctx.sepc;
        state.push_sie();
        // 设置 sstatus 指向 S mode
        state.csrs.sstatus.set_bit(STATUS_SPP_BIT, true);
        ctx.sepc = state.csrs.stvec;
    }else{
        state.interrupt = false;
    }
}

/// 向 guest kernel 转发异常
pub fn forward_exception<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>, ctx: &mut TrapContext) {
    let state = &mut guest.shadow_state;
    state.csrs.scause = scause::read().code();
    state.csrs.sepc = ctx.sepc;
    state.csrs.stval = stval::read();
    // 设置 sstatus 指向 S mode
    state.csrs.sstatus.set_bit(STATUS_SPP_BIT, true);
    ctx.sepc = state.csrs.stvec;
    // 将当前中断上下文修改为中断处理地址，以便陷入内核处理
    match guest.shadow_state.smode() {
        true => {},
        false => {}
    }
}