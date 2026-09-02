

use riscv::addr::BitField;
use super::TrapContext;
use super::forward_exception;
use crate::debug::PageDebug;
use crate::constants::csr::status::STATUS_SPP_BIT;
use crate::page_table::PageTable;
use crate::sbi::{ console_putchar, set_timer, console_getchar, shutdown };
use crate::guest::sbi::{ SBI_CONSOLE_GETCHAR, SBI_CONSOLE_PUTCHAR, SBI_SET_TIMER, SBI_SHUTDOWN };
use crate::guest::{Vcpu, VirtualInterrupt};



/// 处理特权级指令问题
pub fn ifault<P: PageTable + PageDebug>(guest: &mut Vcpu<P>, ctx: &mut TrapContext) {
    let (len, inst) = decode_instruction_at_address(guest, ctx.sepc);
    if let Some(inst) = inst {
        match inst {
            riscv_decode::Instruction::Ecall => {
                // PR #22 (fix-bug/route-user-ecalls-to-guest): legacy SBI calls are
                // valid only from virtual S-mode; U-mode ecalls are guest syscalls.
                if !guest.smode {
                    forward_exception(guest, ctx);
                    return;
                }
                match ctx.x[17]  {
                    SBI_SET_TIMER => {
                        let stime = ctx.x[10];
                        guest.shadow_state.csrs.mtimecmp = stime;
                        set_timer(stime);
                        // PR #40 deasserts only this vCPU's virtual timer when
                        // the Guest programs a new deadline.
                        guest.clear_virtual_interrupt(VirtualInterrupt::Timer);
                    }
                    SBI_CONSOLE_PUTCHAR => {
                        let c = ctx.x[10];
                        console_putchar(c);
                    }
                    SBI_CONSOLE_GETCHAR => {
                        let c = console_getchar();
                        ctx.x[10] = c;
                    }
                    SBI_SHUTDOWN => shutdown(),
                    _ => {
                        // hdebug!("forward exception: sepc -> {:#x}", ctx.sepc);
                        forward_exception(guest, ctx);
                        return;
                    }
                }
            },
            riscv_decode::Instruction::Csrrc(i) => {
                let mask = read_register(ctx, i.rs1() as usize);
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let Some(val) = read_guest_csr(guest, ctx, csr) else {
                    return;
                };
                if mask != 0 && !write_guest_csr(guest, ctx, csr, val & !mask) {
                    return;
                }
                write_register(ctx, rd, val);
            }
            riscv_decode::Instruction::Csrrs(i) => {
                let mask = read_register(ctx, i.rs1() as usize);
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let Some(val) = read_guest_csr(guest, ctx, csr) else {
                    return;
                };
                if mask != 0 && !write_guest_csr(guest, ctx, csr, val | mask) {
                    return;
                }
                write_register(ctx, rd, val);
            }
            // 写 CSR 指令
            riscv_decode::Instruction::Csrrw(i) => {
                let csr = i.csr() as usize;
                let Some(prev) = read_guest_csr(guest, ctx, csr) else {
                    return;
                };
                // 向 Shadow CSR 写入
                let val = read_register(ctx, i.rs1() as usize);
                if !write_guest_csr(guest, ctx, csr, val) {
                    return;
                }
                write_register(ctx, i.rd() as usize, prev);
            },
            riscv_decode::Instruction::Csrrwi(i) => {
                let csr = i.csr() as usize;
                let Some(prev) = read_guest_csr(guest, ctx, csr) else {
                    return;
                };
                if !write_guest_csr(guest, ctx, csr, i.zimm() as usize) {
                    return;
                }
                write_register(ctx, i.rd() as usize, prev);
            }
            riscv_decode::Instruction::Csrrsi(i) => {
                let csr = i.csr() as usize;
                let Some(prev) = read_guest_csr(guest, ctx, csr) else {
                    return;
                };
                let mask = i.zimm() as usize;
                if mask != 0 && !write_guest_csr(guest, ctx, csr, prev | mask) {
                    return;
                }
                write_register(ctx, i.rd() as usize, prev);
            },
            riscv_decode::Instruction::Csrrci(i) => {
                let csr = i.csr() as usize;
                let Some(prev) = read_guest_csr(guest, ctx, csr) else {
                    return;
                };
                let mask = i.zimm() as usize;
                if mask != 0 && !write_guest_csr(guest, ctx, csr, prev & !mask) {
                    return;
                }
                write_register(ctx, i.rd() as usize, prev);
            }
            riscv_decode::Instruction::Sret => {
                // PR #18 (fix-bug/smode-interrupt-forwarding): SPP is the return
                // mode. Track the current mode separately before clearing SPP.
                let return_to_smode = guest.shadow_state.csrs.sstatus
                    .get_bit(STATUS_SPP_BIT);
                guest.shadow_state.pop_sie();
                ctx.sepc = guest
                    .get_csr(crate::constants::csr::sepc)
                    .expect("virtual sepc CSR is unavailable");
                guest.shadow_state.csrs.sstatus.set_bit(STATUS_SPP_BIT, false);
                guest.smode = return_to_smode;
                // hdebug!("sret: spec -> {:#x}", ctx.sepc);
                return;
            }
            riscv_decode::Instruction::SfenceVma(i) => {
                // PR #48 (`fix-bug/guest-exception-forwarding`) accepts both
                // global and address-selective Guest fences. Trapped PTE
                // writes already update shadow leaves; marking every cached
                // ASID dirty conservatively supplies the required Host fence.
                let _guest_address = read_register(ctx, i.rs1() as usize);
                guest
                    .shadow_state
                    .shadow_page_tables
                    .mark_all_tlb_dirty();
            }
            riscv_decode::Instruction::Wfi => {}
            _ => {
                // PR #48 lets the Guest kernel decide how to handle a legal
                // trap cause that Hypocaust does not virtualize.
                forward_exception(guest, ctx);
                return;
            }
        }
    }else{
        // PR #48 must not advance from the newly installed Guest trap vector.
        // The old code added `len` after forwarding and entered stvec+2/4.
        forward_exception(guest, ctx);
        return;
    }
    ctx.sepc += len;
}

#[inline]
fn read_register(ctx: &TrapContext, register: usize) -> usize {
    if register == 0 { 0 } else { ctx.x[register] }
}

#[inline]
fn write_register(ctx: &mut TrapContext, register: usize, value: usize) {
    if register != 0 {
        ctx.x[register] = value;
    }
}

/// PR #48 converts unsupported CSR accesses back into the Guest-visible
/// illegal-instruction exception that caused the emulation attempt.
fn read_guest_csr<P: PageTable + PageDebug>(
    guest: &mut Vcpu<P>,
    ctx: &mut TrapContext,
    csr: usize,
) -> Option<usize> {
    match guest.get_csr(csr) {
        Some(value) => Some(value),
        None => {
            forward_exception(guest, ctx);
            None
        }
    }
}

fn write_guest_csr<P: PageTable + PageDebug>(
    guest: &mut Vcpu<P>,
    ctx: &mut TrapContext,
    csr: usize,
    value: usize,
) -> bool {
    if guest.set_csr(csr, value) {
        true
    } else {
        forward_exception(guest, ctx);
        false
    }
}

/// decode instruction from Guest OS address
pub fn decode_instruction_at_address<P: PageTable + PageDebug>(guest: &Vcpu<P>, addr: usize) -> (usize, Option<riscv_decode::Instruction>) {
    // PR #48 forwards an instruction access failure into the Guest instead of
    // unwrapping a missing shadow translation and panicking the Host.
    let Some(paddr) = guest.translate_guest_vaddr(addr) else {
        return (0, None);
    };
    let i1 = unsafe{ core::ptr::read(paddr as *const u16) };
    let len = riscv_decode::instruction_length(i1);
    let inst = match len {
        2 => i1 as u32,
        4 => unsafe{ core::ptr::read(paddr as *const u32) },
        _ => return (len, None),
    };
    (len, riscv_decode::decode(inst).ok())
}
