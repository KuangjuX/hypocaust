# VM-owned Guest memory

PR #36 (`feature/vm-guest-memory`) replaces address arithmetic spread across
the hypervisor with a checked `GuestMemory` capability owned by each VM.

## Address spaces

Hypocaust has three relevant address spaces while shadow paging is active:

| Address | Meaning | Owner |
| --- | --- | --- |
| GPA | Address visible to the Guest | VM |
| HPA | Host physical backing for Guest RAM | Hypocaust |
| shadow HPA | Host backing for synchronized shadow page tables | VM |

The initial implementation reserves three fixed 128 MiB VM slots. This is an
explicit transitional platform limit, not a hart count: a vCPU can execute on
any physical hart without changing its translations. The later multi-Guest
configuration PR will validate these slots against the platform memory map.

## Translation boundary

`GuestMemory` exposes checked single-address and range translations. Every
translation rejects subtraction/addition overflow and access beyond the VM's
RAM or shadow slot. Page-table code validates an entire page before treating
it as a PTE array. VirtIO validates the complete legacy vring and every DMA
descriptor range before an HPA is passed to QEMU.

The Guest ELF loader also uses the VM-owned Host slot and rejects:

- a load segment outside Guest RAM;
- a file size larger than its memory size;
- a truncated file range;
- an aligned segment allocation that exceeds Host backing RAM; and
- address arithmetic overflow.

## Ownership

`VirtualMachine` owns an immutable, reference-counted `GuestMemory`
capability. Its vCPUs share that capability because page-table emulation runs
in vCPU trap context. Consumers can request translations, but cannot mutate or
reconstruct another VM's slot.

This PR originally left vCPU mappings and virtual devices in their existing
location. PR #39 moved the device model to a per-VM bus, and PR #47
(`feature/iommu-passthrough-policy`) now classifies the checked QEMU DMA path
as mediated. Real passthrough requires a separate IOMMU-protected adapter.
