//! Implementation of [`PageTableEntry`] and [`PageTable`].

use super::pte::PteWrapper;
use super::{PhysPageNum, StepByOne, VirtAddr, VirtPageNum, PTEFlags, PageTableEntry, PageTable, PageTableLevel};
use crate::guest::gpa2hpa;
use crate::hyp_alloc::{FrameTracker, frame_alloc};
use alloc::vec;
use alloc::vec::Vec;



/// page table structure
#[derive(Clone)]
pub struct PageTableSv39 {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable for PageTableSv39 {
    fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTableSv39 {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    /// Temporarily used to get arguments from user space.
    fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }

    fn from_ppn(ppn: PhysPageNum) -> Self {
        Self {
            root_ppn: ppn,
            frames: Vec::new()
        }
    }

    fn root_ppn(&self) -> PhysPageNum {
        self.root_ppn
    }

    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }

    fn find_guest_pte(&self, vpn: VirtPageNum, hart_id: usize) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte;
            if i == 0{ pte = &mut ppn.get_pte_array()[*idx]; }
            else{ 
                ppn = PhysPageNum::from(gpa2hpa(ppn.0 << 12, hart_id) >> 12);
                pte = &mut ppn.get_pte_array()[*idx]; 
            }
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }

    #[allow(unused)]
    fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    #[allow(unused)]
    fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    #[allow(unused)]
    fn try_map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        match self.translate(vpn) {
            Some(pte) => {
                if !pte.is_valid(){ self.map(vpn, ppn, flags) }
            },
            None => { self.map(vpn, ppn, flags) }
        }
    }

    fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }

    #[allow(unused)]
    fn translate_guest(&self, vpn: VirtPageNum, hart_id: usize) -> Option<PageTableEntry> {
        self.find_guest_pte(vpn, hart_id).map(|pte| *pte)
    }

    fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }

    fn walk_page_table<R: Fn(usize) -> usize>(root: usize, va: usize, read_pte: R) -> Option<super::PageWalk> {
        let mut path = Vec::new();
        let mut page_table = root;
        for level in 0..3 {
            let pte_index = (va >> (30 - 9 * level)) & 0x1ff;
            let pte_addr = page_table + pte_index * 8;
            let pte = read_pte(pte_addr);
            let level = match level {
                0 => PageTableLevel::Level1GB,
                1 => PageTableLevel::Level2MB,
                2 => PageTableLevel::Level4KB,
                _ => unreachable!(),
            };
            let pte = PageTableEntry{ bits: pte};
            path.push(PteWrapper{ addr: pte_addr, pte, level});

            if !pte.is_valid() || (pte.writable() && !pte.readable()){ return None; }
            else if pte.readable() | pte.executable() {
                let pa = match level {
                    PageTableLevel::Level4KB => ((pte.bits >> 10) << 12) | (va & 0xfff),
                    PageTableLevel::Level2MB => ((pte.bits >> 19) << 21) | (va & 0x1fffff),
                    PageTableLevel::Level1GB => ((pte.bits >> 28) << 30) | (va & 0x3fffffff),
                };
                return Some(super::PageWalk { path, pa});
            }else{
                page_table = (pte.bits >> 10) << 12;
            }
        }
        None
    }
}

/// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTableSv39::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}



