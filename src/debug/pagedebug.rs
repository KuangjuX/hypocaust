use crate::page_table::{PageTableEntry, PageTable, PhysPageNum};

impl PageTable {
    pub fn print_page_table(&self) {
        let root_pte_array = self.root_ppn().get_pte_array();
        hdebug!("print page table: ");
        print_page_table(root_pte_array, 3);
    }

    pub fn print_guest_page_table(&self) {
        let root_pte_array = self.root_ppn().get_pte_array();
        hdebug!("print guest page table: ");
        print_guest_page_table(root_pte_array, 3);
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