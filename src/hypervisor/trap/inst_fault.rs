

use riscv::addr::BitField;
use riscv::register::scause;

use super::TrapContext;
use super::forward_exception;
use crate::debug::PageDebug;
use crate::constants::csr::sip::STIP_BIT;
use crate::constants::csr::status::STATUS_SPP_BIT;
use crate::page_table::PageTable;
use crate::sbi::{ console_putchar, set_timer, console_getchar, shutdown };
use crate::guest::sbi::{ SBI_CONSOLE_GETCHAR, SBI_CONSOLE_PUTCHAR, SBI_SET_TIMER, SBI_SHUTDOWN };
use crate::guest::GuestKernel;



/// 处理特权级指令问题
pub fn ifault<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>, ctx: &mut TrapContext) {
    let (len, inst) = decode_instruction_at_address(guest, ctx.sepc);
    if let Some(inst) = inst {
        match inst {
            riscv_decode::Instruction::Ecall => {
                match ctx.x[17]  {
                    SBI_SET_TIMER => {
                        let stime = ctx.x[10];
                        guest.shadow_state.csrs.mtimecmp = stime;
                        set_timer(stime);
                        guest.shadow_state.csrs.sip.set_bit(STIP_BIT, false);
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
                let mask = ctx.x[i.rs1() as usize];
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let val = guest.get_csr(csr);
                if mask != 0 {
                    guest.set_csr(csr, val & !mask);
                }
                ctx.x[rd] = val;
            }
            riscv_decode::Instruction::Csrrs(i) => {
                let mask = ctx.x[i.rs1() as usize];
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let val = guest.get_csr(csr);
                if mask != 0 {
                    guest.set_csr(csr, val | mask);
                }
                ctx.x[rd] = val;
            }
            // 写 CSR 指令
            riscv_decode::Instruction::Csrrw(i) => {
                let prev = guest.get_csr(i.csr() as usize);
                // 向 Shadow CSR 写入
                let val = ctx.x[i.rs1() as usize];
                guest.set_csr(i.csr() as usize, val);
                ctx.x[i.rd() as usize] = prev;
            },
            riscv_decode::Instruction::Csrrwi(i) => {
                let prev = guest.get_csr(i.csr() as usize);
                guest.set_csr(i.csr() as usize, i.zimm() as usize);
                ctx.x[i.rd() as usize] = prev;
            }
            riscv_decode::Instruction::Csrrsi(i) => {
                let prev = guest.get_csr(i.csr() as usize);
                let mask = i.zimm() as usize;
                if mask != 0 {
                    guest.set_csr(i.csr() as usize, prev | mask);
                }
                ctx.x[i.rd() as usize] = prev;
            },
            riscv_decode::Instruction::Csrrci(i) => {
                let prev = guest.get_csr(i.csr() as usize);
                let mask = i.zimm() as usize;
                if mask != 0 {
                    guest.set_csr(i.csr() as usize, prev & !mask);
                }
                ctx.x[i.rd() as usize] = prev;
            }
            riscv_decode::Instruction::Sret => {
                guest.shadow_state.pop_sie();
                ctx.sepc = guest.get_csr(crate::constants::csr::sepc);
                guest.shadow_state.csrs.sstatus.set_bit(STATUS_SPP_BIT, false);
                if !guest.shadow_state.smode() {
                    guest.shadow_state.interrupt = true;
                }
                // hdebug!("sret: spec -> {:#x}", ctx.sepc);
                return;
            }
            riscv_decode::Instruction::SfenceVma(i) => {
                if i.rs1() == 0 {
                    // unsafe{ core::arch::asm!("sfence.vma") };
                }else{
                    unimplemented!()
                }
            }
            riscv_decode::Instruction::Wfi => {}
            _ => {
                let paddr = guest.translate_guest_vaddr(ctx.sepc).unwrap();
                let inst = unsafe{ core::ptr::read(paddr as *const u32) };
                panic!("Unrecognized instruction, sepc: {:#x}, scause: {:?}, inst: {:#x}", ctx.sepc, scause::read().cause(), inst)
            }
        }
    }else{ 
        forward_exception(guest, ctx) 
    }
    ctx.sepc += len;
}

/// decode instruction from Guest OS address
pub fn decode_instruction_at_address<P: PageTable + PageDebug>(guest: &GuestKernel<P>, addr: usize) -> (usize, Option<riscv_decode::Instruction>) {
    let paddr = guest.translate_guest_vaddr(addr).unwrap();
    let i1 = unsafe{ core::ptr::read(paddr as *const u16) };
    let len = riscv_decode::instruction_length(i1);
    let inst = match len {
        2 => i1 as u32,
        4 => unsafe{ core::ptr::read(paddr as *const u32) },
        _ => unreachable!()
    };
    (len, riscv_decode::decode(inst).ok())
}










