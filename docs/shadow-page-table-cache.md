# Shadow page-table synchronization cache

The cache added by `feature/cache-shadow-page-table-state` avoids rebuilding
the same synchronization evidence on every cached `satp` switch.

## Consistency model

Hypocaust already traps writes to guest page-table pages and mirrors each PTE
write into shadow memory. The cache adds two monotonically increasing values:

- one guest-wide PTE generation, incremented after every trapped PTE write;
- one synchronized generation stored with each cached shadow root.

A cached root whose generation matches the guest generation can be selected
without walking the guest page tables. If the generations differ, Hypocaust
keeps the previous conservative behavior: a kernel root revisits and protects
all page-table pages, while a user root also fully synchronizes its shadow
tree. The root is then marked synchronized at the current generation.

This guest-wide generation deliberately over-invalidates: a write affecting
one address space makes every cached root stale. It does not risk reusing stale
state, and repeated switches between PTE writes become cheap. A later reverse
mapping from page-table pages to roots could narrow invalidation further.

Page-table mode is still classified from the live guest root on every switch.
Only synchronization freshness is cached, so reusing a root PPN cannot leave a
stale GVA/UVA classification behind.

## xv6-rust result

Both samples used the same debug build, QEMU configuration, guest image, and
128 cumulative Sv39 `satp` updates:

| Metric | Before | With generation cache | Reduction |
| --- | ---: | ---: | ---: |
| Full page-table walks | 188 | 10 | 94.7% |
| Page-table pages visited | 13,676 | 846 | 93.8% |
| PTEs examined | 7,002,112 | 433,152 | 93.8% |
| Shadow-update cycles | 4,278,233,000 | 336,024,000 | 92.1% |
| Average cycles per `satp` update | 33,423,695 | 2,625,187 | 92.1% |

The run reached the xv6-rust shell and successfully executed
`echo CACHE_MODE_SAFE_OK`. Cycle values are QEMU/debug measurements rather
than hardware benchmarks; traversal counts are the primary deterministic
comparison.
