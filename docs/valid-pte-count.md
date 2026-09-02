# Incremental valid-PTE counts

PR #27 (`feature/track-valid-pte-count`) removes a linear scan from the trapped
guest PTE-write path.

## Previous behavior

Guest page-table pages are write-protected in the shadow mapping. When xv6-rust
writes a PTE, Hypocaust traps, emulates the store, mirrors the new entry, and
decides whether that physical page is still a page table. An invalid PTE used
to trigger a scan of all 512 entries on the page; only after finding no valid
entries could Hypocaust safely make the page writable again.

That scan was correct but redundant. Full shadow synchronization already reads
every PTE and trapped writes provide both the old and new V bits.

## Incremental state

Each guest-physical page-table page now has a count of entries whose V bit is
set. Existing full walks calculate and refresh the count while they perform
their required traversal, so initialization does not add another pass.

On a trapped PTE write, Hypocaust reads the old entry before emulating the
store, then updates the count in constant time:

- invalid to valid increments the count;
- valid to invalid decrements the count;
- writes that do not change V leave the count unchanged.

The shadow mapping becomes writable only when the resulting count is zero. If
a write reaches a page not yet observed by a full walk, Hypocaust scans that
single page once after applying the write, records its current count, and uses
the incremental path thereafter. The profiling counter therefore measures
fallback scans rather than every invalid-PTE write.

## xv6-rust result

The same xv6-rust workload reached the shell, executed
`echo VALID_PTE_COUNT_OK`, processed 268 incremental PTE updates by the
64-`satp` sample, and performed zero fallback scans. The comparable previous
implementation scanned 512 entries for each invalid-PTE update.

The debug and release configurations both build with the embedded xv6-rust
kernel. Cycle values under QEMU vary between runs, so the deterministic result
is the removal of all observed fallback scans without changing the number of
incremental PTE updates.
