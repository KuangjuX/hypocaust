use alloc::collections::VecDeque;

use crate::{page_table::PageTable, debug::PageDebug, guest::PageTableRoot};

/// Keeps information for all guest page tables in the system
pub struct PageTrackerInner<P: PageTable + PageDebug> {
    pub mode: PageTableRoot,
    pub page_table: P,
    pub satp: usize
}

pub struct PageTracker<P: PageTable + PageDebug> {
    pub page_tables: VecDeque<PageTrackerInner<P>>
}

impl<P: PageTable + PageDebug> PageTracker<P> {
    pub fn new() -> Self {
        Self { 
            page_tables: VecDeque::new()
        }
    }

    pub fn push(&mut self, mode: PageTableRoot, satp: usize, page_table: P) {
        self.page_tables.push_back(PageTrackerInner {mode, page_table, satp});
    }

    pub fn guest_page_table(&self) -> Option<&P> {
        for inner in self.page_tables.iter() {
            if inner.mode == PageTableRoot::GVA{
                return Some(&inner.page_table)
            }
        }
        None
    }
}