use crate::constants::layout::{TRAMPOLINE, TRAP_CONTEXT};
use crate::hypervisor::trap::TrapContext;
use crate::page_table::{VirtPageNum, PageTable};

use super::PageDebug;

#[allow(unused)]
pub fn print_guest_backtrace<P: PageTable + PageDebug>(spt: &P, satp: usize, ctx: &TrapContext) {
    let pc = ctx.sepc;
    let mut ra = ctx.x[1];
    let mut sp = ctx.x[2];
    let mut fp = ctx.x[8];

    let mut old_fp = 0;
    while old_fp != fp {
        hdebug!("ra -> {:#x}", ra);
        ra = match fp.checked_sub(8) {
            Some(va) => {
                let vpn = VirtPageNum::from(va >> 12);
                let offset = va & 0xfff;
                match spt.translate(vpn) {
                    Some(ppn) => {
                        let pa = offset + (ppn.ppn().0 << 12);
                        unsafe{ core::ptr::read(pa as *const usize) }
                    }
                    None => break
                }
            },
            None => break,
        };

        old_fp = fp;

        fp = match fp.checked_sub(16) {
            Some(va) => {
                let vpn = VirtPageNum::from(va >> 12);
                let offset = va & 0xfff;
                match spt.translate(vpn) {
                    Some(ppn) => {
                        let pa = offset + (ppn.ppn().0 << 12);
                        unsafe{ core::ptr::read(pa as *const usize) }
                    }
                    None => break
                }
            },
            None => break,
        };
    }
}

#[allow(unused)]
pub fn print_hypervisor_backtrace(ctx: &TrapContext) {
    let mut ra = ctx.x[1];
    let mut fp = ctx.x[8];
    let mut old_fp = 0;
    while old_fp != fp {
        hdebug!("ra -> {:#x}", ra);
        ra = match fp.checked_sub(8) {
            Some(addr) => {
                if (addr >= 0x8020_0000 && addr <= 0x8800_0000) || (addr >= TRAP_CONTEXT && addr <= TRAMPOLINE) {
                    unsafe{ core::ptr::read(addr as *const usize) }
                }else{
                    break;
                }
            },
            None => break,
        };

        old_fp = fp;

        fp = match fp.checked_sub(16) {
            Some(addr) => {
                if (addr >= 0x8020_0000 && addr <= 0x8800_0000) || (addr >= TRAP_CONTEXT && addr <= TRAMPOLINE) {
                    unsafe{ core::ptr::read(addr as *const usize) }
                }else{
                    break;
                }
            },
            None => break,
        };
    }
}
