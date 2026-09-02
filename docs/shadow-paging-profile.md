# Shadow-paging profile

The profiling counters added by `feature/shadow-paging-profile` measure work
performed while Hypocaust maintains software shadow page tables. They are
reported after power-of-two `satp` update counts during short runs and every
1,024 updates during long runs.

Each `[Tracking] shadow-paging` line contains cumulative values for one guest:

- `traps`: transitions from the deprivileged guest into Hypocaust.
- `satp_updates`: Sv39 `satp` writes emulated by Hypocaust.
- `new`: shadow page tables initialized for previously unseen guest `satp`
  values.
- `cached_kernel` and `cached_user`: switches to existing kernel and user
  shadow page tables.
- `full_walks`: complete guest page-table traversals.
- `walked_pages` and `walked_ptes`: page-table pages and entries examined by
  those traversals.
- `pte_updates`: guest PTE writes synchronized incrementally after a protected
  page-table page faults.
- `invalidation_scans`: incremental invalidations that scan all 512 entries in
  a page-table page.
- `update_cycles`, `average_cycles`, and `max_cycles`: cycles spent in complete
  `make_shadow_page_table` calls.

The cycle totals intentionally exclude serial reporting. Compare measurements
using the same QEMU configuration and build profile; debug and release cycle
counts are not directly comparable.

On the xv6-rust boot baseline, the first kernel shadow page table traversed 204
page-table pages (104,448 PTEs). This confirms that complete page-table walks
are large enough to measure separately before changing the synchronization
algorithm.
