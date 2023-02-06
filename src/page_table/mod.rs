
mod address;
mod pte;
mod sv39;
mod sv48;
mod sv57;

use alloc::vec::Vec;
use crate::guest::gpa2hpa;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use address::{StepByOne, VPNRange, PPNRange, PageRange};
pub use sv39::PageTableSv39;
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
    fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    #[allow(unused)]
    fn translate_guest(&self, vpn: VirtPageNum, hart_id: usize) -> Option<PageTableEntry> {
        self.find_guest_pte(vpn, hart_id).map(|pte| *pte)
    }
    fn token(&self) -> usize;
    /// page walk，并返回所有 `walk` 过的所有页表项
    fn walk_page_table<R: Fn(usize) -> usize>(root: usize, va: usize, read_pte: R) -> Option<PageWalk>;

    fn translate_host_address(&self, va: usize) -> Option<AddressTranslation> {
        let root_page_table = self.root_ppn().0 << 12;
        Self::walk_page_table(root_page_table, va, |pa|{
            unsafe{ core::ptr::read(pa as *const usize) }
        }).map(|t| {
            AddressTranslation {
                pte: t.path[t.path.len() - 1].pte,
                pte_addr: t.path[t.path.len() - 1].addr,
                level: t.path[t.path.len() - 1].level,
                guest_pa: t.pa,
            }
        })
    }

    fn translate_guest_address(&self, va: usize, hart_id: usize) -> Option<AddressTranslation> {
        let root_page_table = self.root_ppn().0 << 12;
        Self::walk_page_table(root_page_table, va, |va|{
            let pa = gpa2hpa(va, hart_id);
            unsafe{ core::ptr::read(pa as *const usize) }
        }).map(|t| {
            AddressTranslation {
                pte: t.path[t.path.len() - 1].pte,
                pte_addr: t.path[t.path.len() - 1].addr,
                level: t.path[t.path.len() - 1].level,
                guest_pa: t.pa,
            }
        })
    }
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


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PageTableLevel {
    Level4KB,
    Level2MB,
    Level1GB,
}

#[derive(Debug)]
pub struct PteWrapper {
    pub addr: usize,
    pub pte: PageTableEntry,
    pub level: PageTableLevel
}

#[derive(Debug)]
pub struct PageWalk {
    pub path: Vec<PteWrapper>,
    pub pa: usize
}

#[derive(Debug)]
pub struct AddressTranslation {
    pub pte: PageTableEntry,
    pub pte_addr: usize,
    pub guest_pa: usize,
    pub level: PageTableLevel
}


