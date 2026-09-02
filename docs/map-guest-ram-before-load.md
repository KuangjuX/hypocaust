# Map Guest RAM before loading

PR #44 (`fix-bug/map-guest-ram-before-load`) fixes a Host page fault when
creating any VM after Host paging has been enabled.

## Root cause

The Guest ELF loader copied bytes directly to the VM's Host RAM slot before
that slot was present in the Host page table. VM 0 accidentally succeeded
because it was constructed while Hypocaust still inherited bare addressing
from firmware. `vm_init` added VM 0's mapping and enabled paging only after the
copy. Loading VM 1 at `0x90000000` then raised a Host `StorePageFault`.

This made Guest creation order and the current Host `satp` state part of an
otherwise VM-local operation.

## Fix

`MemorySet::new_guest_kernel` now establishes and activates the complete
VM-owned Host RAM mapping before parsing or copying the ELF. The mapping is
Host-readable and Host-writable but not Host-executable; Guest execute
permission remains controlled by that vCPU's page tables.

The old post-copy `hyper_load_guest_kernel` step is no longer used by
`vm_init`, which prevents duplicate mappings. The existing entry point remains
available to avoid mixing an API cleanup into this bug fix.

## Validation

```console
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
make qemu SMP=2
```

The single-Guest regression must still boot xv6-rust and initialize its file
system. The multi-Guest feature branch additionally verifies that loading the
second RAM slot no longer faults after Host paging is active.
