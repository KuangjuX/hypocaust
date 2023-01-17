use super::TrapContext;
use crate::sbi::{ console_putchar, SBI_CONSOLE_PUTCHAR };
use crate::guest::{ get_shadow_csr, write_shadow_csr, translate_guest_paddr, satp_handler };

pub fn instruction_handler(ctx: &mut TrapContext) {
    // let gpepc = translate_guest_vaddr(ctx.sepc);
    let gpepc = ctx.sepc;
    // println!("[hypervisor] gpepc: {:#x}", gpepc);
    let vhepc = translate_guest_paddr(gpepc);
    // println!("[hypervisor] vhepc: {:#x}", vhepc);
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
            // riscv_decode::Instruction::Sd(i) => {
            //     let rs1 = i.rs1() as usize;
            //     let rs2 = i.rs2() as usize;
            //     let offset: isize = if i.imm() > 2048 {
            //         ((0b1111 << 12) | i.imm()) as i16 as isize
            //     }else{ 
            //         i.imm() as isize
            //     };
            //     // 将虚拟地址转换成物理地址，这里地址是相同的，当加入 guest 分页后需要进行影子页表映射
            //     let guest_vaddr = ctx.x[rs1] as isize + offset; 
            //     // let guest_paddr = translate_guest_vaddr(guest_vaddr as usize);
            //     // 将 x[rs2] 的值写入内存中
            //     unsafe{
            //         core::ptr::write(guest_paddr as *mut usize, ctx.x[rs2]);
            //     }
            // },
            // riscv_decode::Instruction::Sw(i) => {
            //     let rs1 = i.rs1() as usize;
            //     let rs2 = i.rs2() as usize;
            //     let offset: isize = if i.imm() > 2048 {
            //         ((0b1111 << 12) | i.imm()) as i16 as isize
            //     }else{ 
            //         i.imm() as isize
            //     };
            //     // 将虚拟地址转换成物理地址，这里地址是相同的，当加入 guest 分页后需要进行影子页表映射
            //     let guest_vaddr = ctx.x[rs1] as isize + offset; 
            //     let guest_paddr = translate_guest_vaddr(guest_vaddr as usize);
            //     // 将 x[rs2] 的值写入内存中
            //     unsafe{
            //         core::ptr::write(guest_paddr as *mut u32, (ctx.x[rs2] & 0xffff_ffff) as u32);
            //     }
            // }
            // riscv_decode::Instruction::Sb(i) => {
            //     let rs1 = i.rs1() as usize;
            //     let rs2 = i.rs2() as usize;
            //     let offset: isize = if i.imm() > 2048 {
            //         ((0b1111 << 12) | i.imm()) as i16 as isize
            //     }else{ 
            //         i.imm() as isize
            //     };
            //     // 将虚拟地址转换成物理地址，这里地址是相同的，当加入 guest 分页后需要进行影子页表映射
            //     let guest_vaddr = ctx.x[rs1] as isize + offset; 
            //     let guest_paddr = translate_guest_vaddr(guest_vaddr as usize);
            //     // 将 x[rs2] 的值写入内存中
            //     unsafe{
            //         core::ptr::write(guest_paddr as *mut u8, (ctx.x[rs2] & 0xff) as u8);
            //     }
            // }
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
            riscv_decode::Instruction::SfenceVma(i) => {
                if i.rs1() == 0 {

                }else{
                    panic!("[hypervisor] Unimplented!");
                }
            }
            riscv_decode::Instruction::Wfi => {}
            _ => { panic!("[hypervisor] Unrecognized instruction, spec: {:#x}", ctx.sepc)}
        }
    }else{ panic!("[hypervisor] Failed to parse instruction, sepc: {:#x}", ctx.sepc) }
    ctx.sepc += len;
}

/// 向 guest kernel 转发异常
pub fn forward_expection(_ctx: &mut TrapContext) {
    
}

/// 向 guest kernel 转发中断
pub fn forward_interrupt(_ctx: &mut TrapContext) {
    
}