use crate::guest::GuestKernel;
use crate::trap::TrapContext;
use crate::mm::VirtPageNum;

#[allow(unused)]
pub fn print_guest_backtrace(guest: &GuestKernel, ctx: &TrapContext) {
    let pc = ctx.sepc;
    let mut ra = ctx.x[1];
    let mut sp = ctx.x[2];
    let mut fp = ctx.x[8];
    // hdebug!("pc -> {:#x}, ra -> {:#x}, sp -> {:#x}, fp -> {:#x}", pc, ra, sp, fp);
    let satp = guest.shadow_state.csrs.satp;
    let spt = guest.shadow_state.shadow_page_tables.find_shadow_page_table(satp).unwrap();

    let mut old_fp = 0;
    while old_fp != fp {
        hdebug!("ra -> {:#x}", ra);
        ra = match fp.checked_sub(8) {
            Some(va) => {
                let vpn = VirtPageNum::from(va >> 12);
                let offset = va & 0xfff;
                match spt.page_table.translate(vpn) {
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
                match spt.page_table.translate(vpn) {
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
