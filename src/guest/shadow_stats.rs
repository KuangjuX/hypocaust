//! Lightweight counters for measuring software shadow-paging overhead.
//!
//! PR #24 (`feature/shadow-paging-profile`) adds this instrumentation. It
//! keeps serial output out of the measured section and reports only
//! periodically so the profiler does not dominate the cost it observes.

const REPORT_INTERVAL: usize = 1024;

#[derive(Copy, Clone)]
pub enum ShadowPageTableUpdate {
    New,
    CachedKernel,
    CachedUser,
}

pub struct ShadowPagingStats {
    traps: usize,
    satp_updates: usize,
    new_shadow_page_tables: usize,
    cached_kernel_switches: usize,
    cached_user_switches: usize,
    full_walks: usize,
    walked_page_table_pages: usize,
    walked_ptes: usize,
    incremental_pte_updates: usize,
    invalidation_page_scans: usize,
    update_cycles: usize,
    max_update_cycles: usize,
}

impl ShadowPagingStats {
    pub const fn new() -> Self {
        Self {
            traps: 0,
            satp_updates: 0,
            new_shadow_page_tables: 0,
            cached_kernel_switches: 0,
            cached_user_switches: 0,
            full_walks: 0,
            walked_page_table_pages: 0,
            walked_ptes: 0,
            incremental_pte_updates: 0,
            invalidation_page_scans: 0,
            update_cycles: 0,
            max_update_cycles: 0,
        }
    }

    #[inline]
    pub fn record_trap(&mut self) {
        self.traps += 1;
    }

    #[inline]
    pub fn record_pte_update(&mut self, scanned_for_invalidation: bool) {
        self.incremental_pte_updates += 1;
        if scanned_for_invalidation {
            self.invalidation_page_scans += 1;
        }
    }

    pub fn record_satp_update(
        &mut self,
        update: ShadowPageTableUpdate,
        full_walks: usize,
        walked_page_table_pages: usize,
        cycles: usize,
    ) {
        self.satp_updates += 1;
        match update {
            ShadowPageTableUpdate::New => self.new_shadow_page_tables += 1,
            ShadowPageTableUpdate::CachedKernel => self.cached_kernel_switches += 1,
            ShadowPageTableUpdate::CachedUser => self.cached_user_switches += 1,
        }
        self.full_walks += full_walks;
        self.walked_page_table_pages += walked_page_table_pages;
        self.walked_ptes += walked_page_table_pages * 512;
        self.update_cycles = self.update_cycles.wrapping_add(cycles);
        self.max_update_cycles = self.max_update_cycles.max(cycles);

        // Early power-of-two samples make short boot tests useful; long-running
        // guests settle to one report per interval to bound serial I/O overhead.
        if self.satp_updates.is_power_of_two() || self.satp_updates % REPORT_INTERVAL == 0 {
            self.report();
        }
    }

    fn report(&self) {
        let average_cycles = self.update_cycles / self.satp_updates;
        htracking!(
            "shadow-paging traps={} satp_updates={} new={} cached_kernel={} cached_user={} full_walks={} walked_pages={} walked_ptes={} pte_updates={} invalidation_scans={} update_cycles={} average_cycles={} max_cycles={}",
            self.traps,
            self.satp_updates,
            self.new_shadow_page_tables,
            self.cached_kernel_switches,
            self.cached_user_switches,
            self.full_walks,
            self.walked_page_table_pages,
            self.walked_ptes,
            self.incremental_pte_updates,
            self.invalidation_page_scans,
            self.update_cycles,
            average_cycles,
            self.max_update_cycles,
        );
    }
}
