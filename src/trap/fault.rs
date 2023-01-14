use super::TrapContext;
use crate::sbi::{ console_putchar, SBI_CONSOLE_PUTCHAR };
use crate::guest::{ get_shadow_csr, write_shadow_csr };

pub fn instruction_handler(ctx: &mut TrapContext) {
    let epc = ctx.sepc;
    let i1 = unsafe{ core::ptr::read(epc as *const u16) };
    let len = riscv_decode::instruction_length(i1);
    let inst = match len {
        2 => i1 as u32,
        4 => unsafe{ core::ptr::read(epc as *const u32) },
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
            riscv_decode::Instruction::Sd(i) => {
                let rs1 = i.rs1() as usize;
                let rs2 = i.rs2() as usize;
                let offset: isize = if i.imm() > 2048 {
                    ((0b1111 << 12) | i.imm()) as i16 as isize
                }else{ 
                    i.imm() as isize
                };
                // 将虚拟地址转换成物理地址，这里地址是相同的，当加入 guest 分页后需要进行影子页表映射
                let guest_virt_addr = ctx.x[rs1] as isize + offset; 
                let guest_phy_addr = guest_virt_addr;
                // 将 x[rs2] 的值写入内存中
                unsafe{
                    core::ptr::write(guest_phy_addr as *mut usize, ctx.x[rs2]);
                }
            },
            riscv_decode::Instruction::Sw(i) => {
                let rs1 = i.rs1() as usize;
                let rs2 = i.rs2() as usize;
                let offset: isize = if i.imm() > 2048 {
                    ((0b1111 << 12) | i.imm()) as i16 as isize
                }else{ 
                    i.imm() as isize
                };
                // 将虚拟地址转换成物理地址，这里地址是相同的，当加入 guest 分页后需要进行影子页表映射
                let guest_virt_addr = ctx.x[rs1] as isize + offset; 
                let guest_phy_addr = guest_virt_addr;
                // 将 x[rs2] 的值写入内存中
                unsafe{
                    core::ptr::write(guest_phy_addr as *mut u32, (ctx.x[rs2] & 0xffff_ffff) as u32);
                }
            }
            riscv_decode::Instruction::Sb(i) => {
                let rs1 = i.rs1() as usize;
                let rs2 = i.rs2() as usize;
                let offset: isize = if i.imm() > 2048 {
                    ((0b1111 << 12) | i.imm()) as i16 as isize
                }else{ 
                    i.imm() as isize
                };
                // 将虚拟地址转换成物理地址，这里地址是相同的，当加入 guest 分页后需要进行影子页表映射
                let guest_virt_addr = ctx.x[rs1] as isize + offset; 
                let guest_phy_addr = guest_virt_addr;
                // 将 x[rs2] 的值写入内存中
                unsafe{
                    core::ptr::write(guest_phy_addr as *mut u8, (ctx.x[rs2] & 0xff) as u8);
                }
            }
            riscv_decode::Instruction::Csrrc(i) => {
                let csr = i.csr() as usize;
                let rd = i.rd() as usize;
                let val = get_shadow_csr(csr);
                ctx.x[rd] = val;
            }
            riscv_decode::Instruction::Csrrs(i) => {
                let csr = i.csr() as usize;
                let rd= i.rd() as usize;
                let val = get_shadow_csr(csr);
                ctx.x[rd] = val;
            }
            // 写 CSR 指令
            riscv_decode::Instruction::Csrrw(i) => {
                let csr = i.csr() as usize;
                let rs = i.rs1() as usize;
                // 向 Shadow CSR 写入
                let val = ctx.x[rs];
                write_shadow_csr(csr, val);
            }
            _ => { panic!("[hypervisor] Unrecognized instruction, spec: {:#x}", ctx.sepc)}
        }
    }else{ panic!("[hypervisor] Failed to parse instruction.") }
    ctx.sepc += len;
}