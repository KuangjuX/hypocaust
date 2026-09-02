# Non-RAM shadow leaf isolation

PR #35 (`fix-bug/non-ram-shadow-leaves`) prevents Guest page-table leaves
outside the configured 128 MiB Guest RAM window from becoming direct Host
mappings.

Previously every valid leaf PPN was passed through `gpa2hpa`. That arithmetic
is meaningful only for Guest RAM, but xv6-rust also publishes mappings such as
`0x3fe00000`. Adding the VM slot offset produced an unrelated HPA and could
grant access to memory that the VM does not own.

All three shadow-page-table update paths now use one policy:

- a complete 4 KiB Guest RAM page becomes a User-accessible Host leaf;
- integer overflow or a page crossing the RAM boundary produces an invalid
  shadow PTE; and
- MMIO and other non-RAM addresses therefore trap to Hypocaust for routing or
  exception injection.

This applies during initial construction, cached user-table synchronization,
and incremental trapped PTE writes, so no update path can bypass the policy.
