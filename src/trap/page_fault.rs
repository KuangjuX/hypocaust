use riscv::register::{stval};

use crate::page_table::{PageTable,  PageTableEntry};
use crate::debug::PageDebug;
use crate::guest::{GuestKernel, gpa2hpa, PageTableRoot};
use crate::trap::fault::{decode_instruction_at_address, handle_qemu_virt}; 

use super::TrapContext;

pub fn handle_page_fault<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>, ctx: &mut TrapContext) -> bool {
    let shadow = guest.shadow();
    if shadow == PageTableRoot::GPA {
        hdebug!("Page fault without paginf enabled?");
        return false;
    }
    if shadow == PageTableRoot::UVA {
        // 用户态触发异常，进行转发
        return false;
    }
    let guest_va = stval::read();
    let sepc = ctx.sepc;
    let (len, inst) = decode_instruction_at_address(guest, sepc);
    if guest.virt_device.qemu_virt_tester.in_region(guest_va){
        handle_qemu_virt(guest, ctx);
        ctx.sepc += len;
        return true;
    }

    let mut pte = 0;
    if let Some(_translation) = guest.translate_guest_vaddr(guest_va) {
        // 获得翻译后的物理地址
        // hdebug!("translation: {:#x}", translation);
        if let Some(inst) = inst {
            match inst {
                riscv_decode::Instruction::Sd(i) => {
                    let rs1 = i.rs1() as usize;
                    let rs2 = i.rs2() as usize;
                    let offset: isize = if i.imm() > 2048 { ((0b1111 << 12) | i.imm()) as i16 as isize }else{  i.imm() as isize };
                    let vaddr = (ctx.x[rs1] as isize + offset) as usize; 
                    assert_eq!(vaddr, guest_va);
                    let paddr = gpa2hpa(vaddr, guest.index);
                    unsafe{ core::ptr::write(paddr as *mut usize, ctx.x[rs2]); }
                    pte = ctx.x[rs2];
                },
                riscv_decode::Instruction::Sb(i) => {
                    let rs1 = i.rs1() as usize;
                    let rs2 = i.rs2() as usize;
                    let offset: isize = if i.imm() > 2048 { ((0b1111 << 12) | i.imm()) as i16 as isize }else{  i.imm() as isize };
                    let vaddr = (ctx.x[rs1] as isize + offset) as usize; 
                    assert_eq!(vaddr, guest_va);
                    let paddr = gpa2hpa(vaddr, guest.index);
                    unsafe{ core::ptr::write(paddr as *mut usize, ctx.x[rs2]); }
                    pte = ctx.x[rs2];
                }
                _ => { return false }
            }
        }
        let pte = PageTableEntry{ bits: pte };
        // hdebug!("ppn -> {:#x}, flags: {:?}", pte.ppn().0, pte.flags());
        guest.synchronize_page_table(guest_va, pte);
        ctx.sepc += len;
        return true;
    }
    false
}