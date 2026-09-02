# Multi-Guest QEMU execution

PR #45 (`feature/multi-guest-qemu`) boots two isolated xv6-rust instances and
connects each VM to its own QEMU VirtIO block backend.

## Resource ownership

| Resource | VM 0 | VM 1 |
| --- | --- | --- |
| VM RAM Host slot | `0x88000000..0x90000000` | `0x90000000..0x98000000` |
| Global vCPU ID | 0 | 1 |
| Guest hart ID | 0 | 0 |
| Guest VirtIO GPA | `0x10001000` | `0x10001000` |
| Host VirtIO backend | first active QEMU device | second active QEMU device |
| Writable disk | `fs-vm0.img` | `fs-vm1.img` |
| Virtual PLIC | VM-owned | VM-owned |
| Guest DTB | VM-owned final RAM page | VM-owned final RAM page |

The Host parses active VirtIO devices from its firmware DTB and grants exactly
one MMIO backend to each VM. Guests never receive Host physical backend
addresses: both see the standard VirtIO GPA, and trapped register accesses are
routed through their own `DeviceBus`.

## Independent device trees

Hypocaust synthesizes a flattened device tree for every VM and passes its GPA
in the standard RISC-V `a1` boot register. Each tree contains only:

- that VM's 128 MiB RAM;
- Guest hart 0;
- that VM's virtual PLIC;
- one VirtIO MMIO block frontend.

The Host DTB is not copied into Guest RAM. A private `hypocaust,vm-id`
property makes otherwise identical single-vCPU configurations distinguishable
to a Guest that chooses to consume it.

## Disk isolation

`make xv6-rust` copies the xv6-rust filesystem into two writable images.
QEMU attaches them as different block devices, so queue state, used rings,
interrupt state, and filesystem writes are not shared between VMs.

This remains mediated assignment: Hypocaust traps MMIO and validates DMA while
QEMU executes block requests. PR #47 (`feature/iommu-passthrough-policy`)
reserves the term passthrough for an exclusively owned device protected by a
platform IOMMU domain and interrupt remapping; QEMU does not silently enter
that mode.

## Run

```console
make xv6-rust
make qemu SMP=2
```

Successful output includes configuration and scheduling lines for VM 0 and VM
1, VM-labelled asynchronous VirtIO completion progress, and two xv6-rust boot
sequences reaching file-system initialization. PR #49
(`feature/per-vm-console`) buffers and labels each Guest's SBI console records;
VM 0 initially owns the shared physical input focus. OpenSBI's own M-mode
diagnostics remain outside that multiplexer and can still interleave with a
partial prompt. With `SMP=1`, both Guests are time-sliced on one Host hart;
`SMP=2` demonstrates concurrent execution.
