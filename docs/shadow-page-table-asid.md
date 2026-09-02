# Shadow page-table ASIDs

PR #26 (`feature/shadow-page-table-asid`) separates Host and guest shadow
translations into hardware TLB namespaces.

## ASID allocation

Hypocaust reserves ASID 0 for its own page table and bare-mode guest mappings.
Each cached Sv39 shadow root receives a stable ASID from 1 through 65,535. The
ASID is encoded into bits 59:44 of the shadow `satp` token; the shadow root PPN
and page-table contents are unchanged.

Hypocaust targets QEMU's RISC-V `virt` machine, whose Sv39 implementation
provides the 16-bit RV64 ASID field. The allocator fails explicitly instead of
silently reusing an ASID if all nonzero identifiers are exhausted.

## TLB invalidation

Trap entry switches to the Host's ASID 0 and executes `sfence.vma x0, t2`,
where `t2` contains zero but is not the architectural `x0` register. RISC-V
therefore flushes ASID 0 only and preserves nonzero guest shadow ASIDs.

Shadow roots carry a dirty-TLB bit. Full shadow walks and trapped guest PTE
writes conservatively dirty every cached root because shadow page-table pages
may be shared. Before returning to a guest root, Hypocaust issues
`sfence.vma x0, <asid>` only if that root is dirty, then clears its bit. Clean
returns switch `satp` without a fence.

Bare-mode guest mappings share ASID 0 with the Host and therefore retain a
mandatory return fence. This is why the counters are named
`return_tlb_flushes` rather than `shadow_tlb_flushes`.

## xv6-rust result

An xv6-rust shell run reached 639,069 Guest returns while exercising process
creation, PTE updates, and 128 cumulative Sv39 `satp` updates:

- 2,088 returns issued a required ASID-scoped fence;
- 636,981 returns reused a clean destination namespace;
- 99.67% of return-side TLB flushes were avoided;
- the shell successfully executed `echo ASID_TLB_OK`.

The entry path still fences Host ASID 0 on every trap, but it no longer evicts
guest translations. Removing that remaining Host fence requires tracking all
runtime mutations of the Hypocaust page table and is intentionally outside
this PR.
