
mod address;
mod pte;
mod sv39;
mod sv48;
mod sv57;


pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use address::{StepByOne, VPNRange, PPNRange, PageRange};
pub use sv39::{translated_byte_buffer, PageTableSv39};
pub use pte::{PageTableEntry, PTEFlags};

pub trait PageTable: Clone {
    fn new() -> Self;
    fn from_token(satp: usize) -> Self;
    fn from_ppn(ppn: PhysPageNum) -> Self;
    fn root_ppn(&self) -> PhysPageNum;
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry>;
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry>;
    fn find_guest_pte(&self, vpn: VirtPageNum, hart_id: usize) -> Option<&mut PageTableEntry>;
    #[allow(unused)]
    fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags);
    #[allow(unused)]
    fn unmap(&mut self, vpn: VirtPageNum);
    #[allow(unused)]
    fn try_map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags);
    fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry>;
    #[allow(unused)]
    fn translate_guest(&self, vpn: VirtPageNum, hart_id: usize) -> Option<PageTableEntry>;
    fn token(&self) -> usize;
}

#[allow(unused)]
pub enum PageError {

}

/// The page sizes supported by RISC -V
#[allow(unused)]
#[repr(u64)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Hash, Ord)]
pub enum PageSize {
    /// Page
    Size4K = 4 * 1024,
    /// Mega
    Size2M = 2 * 1024 * 1024,
    /// Giga
    Size1G = 1024 * 1204 * 1024,
    /// Tera
    Size512G = 512 * 1024 * 1024 * 1024
}


