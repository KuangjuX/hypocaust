use riscv::register::{stval, scause};

use super::TrapContext;
use crate::constants::layout::PAGE_SIZE;
use crate::sbi::{ console_putchar, SBI_CONSOLE_PUTCHAR };
use crate::guest::{ get_shadow_csr, write_shadow_csr, satp_handler, GUEST_KERNEL_MANAGER, GuestKernel };

/// 处理地址错误问题
pub fn pfault(ctx: &mut TrapContext) {
    // 获取地址错信息
    let stval = stval::read();
    let mut inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let id = inner.run_id;
    let guest = &mut inner.kernels[id];
    if let Some(vhaddr) = guest.translate_valid_guest_vaddr(stval) {
        // 处理地址错误
        panic!("stval: {:#x}, vhaddr: {:#x}, cause: {:?}", stval, vhaddr, scause::read().cause());
    }else{
        // 转发到 Guest OS 处理
        forward_exception(guest, ctx)
    }
}

/// 向 guest kernel 转发异常
pub fn forward_exception(guest: &mut GuestKernel, ctx: &mut TrapContext) {
    hdebug!("forward expection");
    let state = &mut guest.shadow_state;
    state.write_scause(scause::read().code());
    state.write_sepc(ctx.sepc);
    state.write_stval(stval::read());
    let stvec = state.get_stvec();
    // 将当前中断上下文修改为中断处理地址，以便陷入内核处理
    ctx.sepc = stvec;
}

/// 向 guest kernel 转发中断
pub fn maybe_forward_interrupt(_ctx: &mut TrapContext) {
    
}

/// 处理特权级指令问题
pub fn ifault(ctx: &mut TrapContext) {
    let inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let id = inner.run_id;
    let guest = &inner.kernels[id];
    let vhepc = guest.translate_guest_vaddr(ctx.sepc).unwrap();
    drop(inner);
    let i1 = unsafe{ core::ptr::read(vhepc as *const u16) };
    let len = riscv_decode::instruction_length(i1);
    let inst = match len {
        2 => i1 as u32,
        4 => unsafe{ core::ptr::read(vhepc as *const u32) },
        _ => unreachable!()
    };
    if let Ok(inst) = riscv_decode::decode(inst) {
        match inst {
            riscv_decode::Instruction::Ecall => {
                let x17 = ctx.x[17];
                match x17  {
                    SBI_CONSOLE_PUTCHAR => {
                        let c = ctx.x[10];
                        console_putchar(c);
                    },
                    _ => { panic!("[hypervisor] Error env call")}
                }
            }
            riscv_decode::Instruction::Csrrc(i) => {
                let mask = ctx.x[i.rs1() as usize];
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let val = get_shadow_csr(csr);
                if mask != 0 {
                    write_shadow_csr(csr, val & !mask);
                }
                ctx.x[rd] = val;
            }
            riscv_decode::Instruction::Csrrs(i) => {
                let mask = ctx.x[i.rs1() as usize];
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let val = get_shadow_csr(csr);
                if mask != 0 {
                    write_shadow_csr(csr, val | mask);
                }
                ctx.x[rd] = val;
            }
            // 写 CSR 指令
            riscv_decode::Instruction::Csrrw(i) => {
                let csr = i.csr() as usize;
                let rs = i.rs1() as usize;
                // 向 Shadow CSR 写入
                let val = ctx.x[rs];
                match csr {
                    crate::constants::csr::satp => { satp_handler(val) },
                    crate::constants::csr::stvec => { 
                        let mut inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
                        let id = inner.run_id;
                        let guest = &mut inner.kernels[id];
                        let state = &mut guest.shadow_state;
                        let stvec = val - (2 + 2 * id) * PAGE_SIZE;
                        state.write_stvec(stvec);
                    }
                    _ => { write_shadow_csr(csr, val); }
                }
            },
            riscv_decode::Instruction::Csrrwi(i) => {
                let prev = get_shadow_csr(i.csr() as usize);
                write_shadow_csr(i.csr() as usize, i.zimm() as usize);
                ctx.x[i.rd() as usize] = prev;
            }
            riscv_decode::Instruction::Csrrsi(i) => {
                let prev = get_shadow_csr(i.csr() as usize);
                let mask = i.zimm() as usize;
                if mask != 0 {
                    write_shadow_csr(i.csr() as usize, prev | mask);
                }
                ctx.x[i.rd() as usize] = prev;
            },
            riscv_decode::Instruction::Csrrci(i) => {
                let prev = get_shadow_csr(i.csr() as usize);
                let mask = i.zimm() as usize;
                if mask != 0 {
                    write_shadow_csr(i.csr() as usize, prev & !mask);
                }
                ctx.x[i.rd() as usize] = prev;
            }
            riscv_decode::Instruction::Sret => {
                panic!("sret: VMM forward exception finished!");
            }
            riscv_decode::Instruction::SfenceVma(i) => {
                if i.rs1() == 0 {

                }else{
                    panic!("[hypervisor] Unimplented!");
                }
            }
            riscv_decode::Instruction::Wfi => {}
            _ => { panic!("[hypervisor] Unrecognized instruction, spec: {:#x}", ctx.sepc)}
        }
    }else{ 
        let mut inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
        let id = inner.run_id;
        let guest = &mut inner.kernels[id];
        forward_exception(guest, ctx) 
    }
    ctx.sepc += len;
}
