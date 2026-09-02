# Invalid leaf translation semantics

PR #46 (`fix-bug/invalid-leaf-translation`) makes `PageTable::translate`
return `Some` only for a valid leaf PTE.

## Root cause

The Sv39 walker returned the final PTE object as soon as it reached level 0,
without checking the `V` bit. After a page had been unmapped, its intermediate
page-table pages remained allocated, so `translate(vpn)` returned
`Some(PageTableEntry::empty())` instead of `None`.

This is unsafe API behavior because callers naturally use `Option::is_some()`
as the mapping test. PR #45 initially skipped the second Host VirtIO mapping
for exactly this reason and later worked around it by checking `PTE.V` again.

## Fix

Both normal and Guest-aware Sv39 translation now filter invalid final entries.
Callers no longer need to repeat the validity check, and the shadow page-table
assertion for trapped VirtIO MMIO now expects an absent translation.

A bare-metal startup self-test covers the complete transition:

1. a never-mapped VPN returns `None`;
2. mapping it returns the expected PPN;
3. unmapping it leaves intermediate page-table pages allocated but returns
   `None` again.

## Validation

```console
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
make qemu SMP=2
```

The two xv6-rust Guests must retain their independent VirtIO backends and both
reach file-system initialization.
