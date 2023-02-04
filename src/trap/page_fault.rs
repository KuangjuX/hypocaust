use riscv::register::stval;

use crate::page_table::{PageTable,  PageTableEntry};
use crate::debug::{PageDebug, print_guest_backtrace};
use crate::guest::{GuestKernel, gpa2hpa, PageTableRoot};
use crate::trap::fault::{decode_instruction_at_address, handle_qemu_virt}; 

use super::TrapContext;

pub fn handle_page_fault<P: PageTable + PageDebug>(guest: &mut GuestKernel<P>, ctx: &mut TrapContext) -> bool {
    let shadow = guest.shadow();
    if shadow == PageTableRoot::GPA {
        hdebug!("Page fault without paging enabled?");
        return false;
    }
    if shadow == PageTableRoot::UVA {
        // 用户态触发异常，进行转发
        hwarning!("Page fault from U mode?");
        return false;
    }

    // let access = match scause::read().cause() {
    //     scause::Trap::Exception(scause::Exception::InstructionPageFault) => PTEFlags::X,
    //     scause::Trap::Exception(scause::Exception::LoadPageFault) => PTEFlags::R,
    //     scause::Trap::Exception(scause::Exception::StorePageFault) => PTEFlags::W,
    //     _ => unreachable!()
    // };
    let guest_va = stval::read();
    if guest_va % core::mem::size_of::<PageTableEntry>() != 0 {
        hwarning!("guest va: {:#x}, sepc: {:#x}", guest_va, ctx.sepc);
        print_guest_backtrace::<P>(&guest.shadow_state.shadow_page_tables.guest_page_table().unwrap().spt, guest.shadow_state.get_satp(), ctx)
    }
    assert_eq!(guest_va % core::mem::size_of::<PageTableEntry>(), 0);
    let sepc = ctx.sepc;
    let (len, inst) = decode_instruction_at_address(guest, sepc);
    // 处理 `MMIO`
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
                    pte = ctx.x[rs2];
                },
                riscv_decode::Instruction::Sb(_) => {
                    panic!("Unsporrted instruction");
                }
                _ => { return false }
            }
        }
        let pte = PageTableEntry{ bits: pte };       
        let guest_pte_addr = gpa2hpa(guest_va, guest.index);
        // if pte.ppn().0 == 0 {
        //     unsafe{core::ptr::write(guest_pte_addr as *mut u8, pte.bits as u8)}
        // }else{
        //     unsafe{ core::ptr::write(guest_pte_addr as *mut usize, pte.bits)}
        // }
        unsafe{ core::ptr::write(guest_pte_addr as *mut usize, pte.bits)}

        guest.synchronize_page_table(guest_va, pte);
        ctx.sepc += len;
        return true;
    }
    false
}