use crate::page_table::{PageTableEntry, PageTableSv39, PhysPageNum, VirtPageNum, PageTable};
use crate::constants::layout::GUEST_TRAP_CONTEXT;
use crate::trap::TrapContext;

pub trait PageDebug {
    fn print_page_table(&self);
    fn print_guest_page_table(&self);
    fn print_trap_context(&self);
}

impl PageDebug for PageTableSv39 {
    fn print_page_table(&self) {
        let root_pte_array = self.root_ppn().get_pte_array();
        hdebug!("print page table: ");
        print_page_table(root_pte_array, 3);
    }

    fn print_guest_page_table(&self) {
        let root_pte_array = self.root_ppn().get_pte_array();
        hdebug!("print guest page table: ");
        print_guest_page_table(root_pte_array, 3);
    }

    fn print_trap_context(&self) {
        let trap_ctx_ppn = self.translate(VirtPageNum::from(GUEST_TRAP_CONTEXT >> 12)).unwrap().ppn().0;
        hdebug!("trap ctx ppn: {:#x}", trap_ctx_ppn);
        unsafe{
            let trap_ctx = &*((trap_ctx_ppn << 12) as *const TrapContext);
            for i in 0..trap_ctx.x.len() {
                hdebug!("x{} -> {:#x}", i, trap_ctx.x[i]);
            }
            hdebug!("sepc -> {:#x}", trap_ctx.sepc);
            hdebug!("sstatus -> {:#x}", trap_ctx.sstatus.bits());
        }
    }

}

pub fn print_page_table(pte_array: &[PageTableEntry], level: u8) {
    if level == 0 { return; }
    for i in 0..512 {
        let pte = pte_array[i];
        if pte.is_valid() {
            for _ in 0..(3 - level) {
                print!("  ");
            }
            println!("{}: {:#x} {:?}", i, pte.ppn().0, pte.flags());
        }
        if pte.is_valid() {
            assert!(level != 0);
            let pte_array = pte.ppn().get_pte_array();
            print_page_table(pte_array, level - 1);
        }
    }
}

pub fn print_guest_page_table(pte_array: &[PageTableEntry], level: u8) {
    if level == 0 { return; }
    for i in 0..512 {
        let pte = pte_array[i];
        if pte.is_valid() {
            for _ in 0..(3 - level) {
                print!("  ");
            }
            println!("{}: {:#x} {:?}", i, pte.ppn().0, pte.flags());
        }
        if pte.is_valid() {
            assert!(level != 0);
            let ppn = PhysPageNum::from(((pte.ppn().0 << 12) + 0x800_0000) >> 12);
            let pte_array = ppn.get_pte_array();
            print_guest_page_table(pte_array, level - 1);
        }
    }
}