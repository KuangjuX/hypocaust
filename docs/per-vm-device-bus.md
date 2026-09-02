# Per-VM device bus

PR #39 (`feature/per-vm-device-bus`) makes the virtual machine, rather than an
individual vCPU, the owner of device state and MMIO routing.

## Why the ownership boundary matters

A real SMP Guest expects all of its vCPUs to observe the same device registers,
VirtIO queue selection, UART state, and interrupt source. Keeping this state in
each `Vcpu` creates divergent copies and makes correct interrupt routing
impossible. It can also let a global address predicate route one VM's access to
a device intended for another VM.

`VirtualMachine` now owns one `DeviceBus`. Trap handling resolves the current
`VcpuKey`, selects its owning VM, and borrows that VM's vCPU and device bus as
disjoint state for the duration of an emulated MMIO instruction.

## Address routing

The bus is the only component that decides whether a Guest address belongs to
a device. It exposes checked 32-bit read and write operations and returns
`None` or `false` for an unmapped address.

Mediated register regions record two addresses explicitly:

- the Guest physical base exposed in that VM's hardware description;
- the Host physical MMIO base assigned to the backend.

The VirtIO and QEMU test-device paths translate the register offset between
those spaces. Queue and descriptor DMA continues to use the VM-owned checked
`GuestMemory` capability introduced by PR #36.

The original QEMU setup had one physical VirtIO block device. Its default
mediated bus was therefore restricted to VM 0 so constructing another VM
cannot silently grant two Guests the same physical device. The later emulated
VirtIO work provides an independent backend for every VM; PR #47
(`feature/iommu-passthrough-policy`) defines the separate opt-in contract for
exclusively assigned, IOMMU-protected passthrough devices.

## Real-hypervisor model

The intended device pipeline is:

```text
Guest MMIO fault
  -> current VcpuKey
  -> owning VirtualMachine
  -> that VM's DeviceBus
  -> mediated/emulated backend, or IOMMU-protected exclusive passthrough
  -> virtual interrupt targeted at one of that VM's vCPUs
```

Physical interrupts are Host events. The Host acknowledges the physical
controller, lets the selected backend update device state, records a pending
virtual interrupt in the destination vCPU, and wakes that vCPU if necessary.
Guest exceptions remain vCPU-local architectural state and are never broadcast
to other Guests.

## Validation

The following checks exercise both build profiles and the existing xv6-rust
VirtIO block initialization through the new VM-owned route:

```console
git diff --check
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
make qemu SMP=2
```

xv6-rust reaches file-system initialization while the second Host hart remains
online in its scheduler loop.
