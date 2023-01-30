//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

mod address;
mod sv39;
mod pte;


pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use address::{StepByOne, VPNRange, PPNRange};
pub use sv39::{translated_byte_buffer, PageTableSv39};
pub use pte::{PageTableEntry, PTEFlags};

pub trait PageTable: Clone {
    fn new() -> Self;
    fn from_token(satp: usize) -> Self;
    fn from_ppn(ppn: PhysPageNum) -> Self;
    fn root_ppn(&self) -> PhysPageNum;
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry>;
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry>;
    fn find_guest_pte(&self, vpn: VirtPageNum, pgt: &Self) -> Option<&mut PageTableEntry>;
    #[allow(unused)]
    fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags);
    #[allow(unused)]
    fn unmap(&mut self, vpn: VirtPageNum);
    #[allow(unused)]
    fn try_map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags);
    fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry>;
    #[allow(unused)]
    fn translate_gvpn(&self, vpn: VirtPageNum, guest_pgt: &Self) -> Option<PageTableEntry>;
    fn token(&self) -> usize;
}


