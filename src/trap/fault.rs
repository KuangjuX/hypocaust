use riscv::register::{stval, scause};

use super::TrapContext;
use crate::constants::csr::sie::STIE;
use crate::constants::csr::sip::{STIP, SEIP, SSIP};
use crate::mm::PageTableEntry;
use crate::sbi::{ console_putchar, SBI_CONSOLE_PUTCHAR, set_timer, SBI_SET_TIMER };
use crate::guest::GuestKernel;
use crate::timer::{get_time, get_default_timer};

/// 处理特权级指令问题
pub fn ifault(guest: &mut GuestKernel, ctx: &mut TrapContext) {
    let (len, inst) = decode_instruction_at_address(guest, ctx.sepc);
    if let Some(inst) = inst {
        match inst {
            riscv_decode::Instruction::Ecall => {
                let x17 = ctx.x[17];
                match x17  {
                    SBI_SET_TIMER => {
                        let stimer = ctx.x[10];
                        guest.shadow_state.write_mtimecmp(stimer);
                    }
                    SBI_CONSOLE_PUTCHAR => {
                        let c = ctx.x[10];
                        console_putchar(c);
                    },
                    _ => {
                        forward_exception(guest, ctx);
                        return;
                    }
                }
            }
            riscv_decode::Instruction::Csrrc(i) => {
                let mask = ctx.x[i.rs1() as usize];
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let val = guest.read_shadow_csr(csr);
                if mask != 0 {
                    guest.write_shadow_csr(csr, val & !mask);
                }
                ctx.x[rd] = val;
            }
            riscv_decode::Instruction::Csrrs(i) => {
                let mask = ctx.x[i.rs1() as usize];
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let val = guest.read_shadow_csr(csr);
                if mask != 0 {
                    guest.write_shadow_csr(csr, val | mask);
                }
                ctx.x[rd] = val;
            }
            // 写 CSR 指令
            riscv_decode::Instruction::Csrrw(i) => {
                let prev = guest.read_shadow_csr(i.csr() as usize);
                // 向 Shadow CSR 写入
                let val = ctx.x[i.rs1() as usize];
                match i.csr() as usize {
                    crate::constants::csr::satp => { guest.satp_handler(val) },
                    _ => { guest.write_shadow_csr(i.csr() as usize, val); }
                }
                ctx.x[i.rd() as usize] = prev;
            },
            riscv_decode::Instruction::Csrrwi(i) => {
                let prev = guest.read_shadow_csr(i.csr() as usize);
                guest.write_shadow_csr(i.csr() as usize, i.zimm() as usize);
                ctx.x[i.rd() as usize] = prev;
            }
            riscv_decode::Instruction::Csrrsi(i) => {
                let prev = guest.read_shadow_csr(i.csr() as usize);
                let mask = i.zimm() as usize;
                if mask != 0 {
                    guest.write_shadow_csr(i.csr() as usize, prev | mask);
                }
                ctx.x[i.rd() as usize] = prev;
            },
            riscv_decode::Instruction::Csrrci(i) => {
                let prev = guest.read_shadow_csr(i.csr() as usize);
                let mask = i.zimm() as usize;
                if mask != 0 {
                    guest.write_shadow_csr(i.csr() as usize, prev & !mask);
                }
                ctx.x[i.rd() as usize] = prev;
            }
            riscv_decode::Instruction::Sret => {
                ctx.sepc = guest.read_shadow_csr(crate::constants::csr::sepc);
                return;
            }
            riscv_decode::Instruction::SfenceVma(i) => {
                if i.rs1() == 0 {
                    unsafe{ core::arch::asm!("sfence.vma") };
                }else{
                    panic!("[hypervisor] Unimplented!");
                }
            }
            riscv_decode::Instruction::Wfi => {}
            _ => { panic!("[hypervisor] Unrecognized instruction, spec: {:#x}", ctx.sepc)}
        }
    }else{ 
        forward_exception(guest, ctx) 
    }
    ctx.sepc += len;
}

/// decode instruction from Guest OS address
pub fn decode_instruction_at_address(guest: &GuestKernel, addr: usize) -> (usize, Option<riscv_decode::Instruction>) {
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


/// 处理地址错误问题
pub fn pfault(guest: &mut GuestKernel, ctx: &mut TrapContext) {
    // 获取地址错信息
    let stval = stval::read();
    if let Some(vhaddr) = guest.translate_valid_guest_vaddr(stval) {
        // 处理地址错误
        if guest.is_guest_page_table(stval) {
            // 检测到 Guest OS 修改页表
            let (len, inst) = decode_instruction_at_address(guest, ctx.sepc);
            if let Some(inst) = inst {
                match inst {
                    riscv_decode::Instruction::Sd(i) => {
                        let rs1 = i.rs1() as usize;
                        let rs2 = i.rs2() as usize;
                        let offset: isize = if i.imm() > 2048 {
                            ((0b1111 << 12) | i.imm()) as i16 as isize
                        }else{ 
                            i.imm() as isize
                        };
                        let vaddr = (ctx.x[rs1] as isize + offset) as usize; 
                        let paddr = guest.gpa2hpa(vaddr);
                        unsafe{
                            core::ptr::write(paddr as *mut usize, ctx.x[rs2]);
                        }
                        guest.sync_shadow_page_table(vaddr, PageTableEntry{ bits: ctx.x[rs2]});
                    },
                    _ => panic!("sepc: {:#x}, stval: {:#x}", ctx.sepc, stval)
                }
            }
            // panic!();
            ctx.sepc += len;
            return;
        }
        panic!("stval: {:#x}, vhaddr: {:#x}, sepc: {:#x}, cause: {:?}", stval, vhaddr, ctx.sepc, scause::read().cause());
    }else{
        // 转发到 Guest OS 处理
        forward_exception(guest, ctx)
    }
}

/// 时钟中断处理函数
pub fn timer_handler(guest: &mut GuestKernel, ctx: &mut TrapContext) {
    let time = get_time();
    let mut next = time + get_default_timer();

    if guest.shadow_state.get_sie() & STIE != 0 {
        if guest.shadow_state.get_mtimecmp() <= time {
            // 表明此时 Guest OS 发生中断
            guest.shadow_state.interrupt = true;
            // 设置 sip 寄存器
            guest.shadow_state.csrs.sip |= STIP;
        }else{
            // 未发生中断，设置下次中断
            next = next.min(guest.shadow_state.get_mtimecmp())
        }
    }
    // 设置下次中断
    set_timer(next);
    maybe_forward_interrupt(guest, ctx);
}

/// 向 guest kernel 转发异常
pub fn forward_exception(guest: &mut GuestKernel, ctx: &mut TrapContext) {
    let state = &mut guest.shadow_state;
    state.write_scause(scause::read().code());
    state.write_sepc(ctx.sepc);
    state.write_stval(stval::read());
    ctx.sepc = state.get_stvec();
    // 将当前中断上下文修改为中断处理地址，以便陷入内核处理
    match guest.shadow_state.smode() {
        true => {},
        false => {}
    }
}

/// 检测客户端是否发生中断，若有则进行转发
pub fn maybe_forward_interrupt(guest: &mut GuestKernel, ctx: &mut TrapContext) {
    // 没有发生中断，返回
    if !guest.shadow_state.interrupt{ return }
    let state = &mut guest.shadow_state;
    if state.get_sie() & state.csrs.sip != 0 {
        // 如果开启中断且有中断正在等待
        let mut cause: usize = if state.csrs.sip & SEIP != 0 { 9 }
        else if state.csrs.sip & STIP != 0 { 5 }
        else if state.csrs.sip & SSIP != 0 { 1 }
        else{ unreachable!() };
        cause = (1 << 63) | cause;
        state.write_scause(cause);
    }else{
        state.interrupt = false;
    }
    state.write_sepc(ctx.sepc);
    ctx.sepc = state.get_stvec();
}

