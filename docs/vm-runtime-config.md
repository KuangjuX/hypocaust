# VM runtime configuration

PR #43 (`feature/vm-runtime-config`) removes two assumptions that prevented
safe multi-Guest construction: a global vCPU number was also treated as the
Guest's hart number, and every VM implicitly selected QEMU's first VirtIO MMIO
device.

## Identity domains

- `VcpuId` is globally unique. Hypocaust uses it for scheduler registration
  and Host kernel-stack allocation.
- `GuestHartId` is unique only inside one VM. It is passed in `a0` at Guest
  boot and selects that VM's virtual PLIC context.
- `HartId` still identifies the physical Host hart currently running a vCPU.

This separation allows VM 0/vCPU 0 and VM 1/vCPU 1 to both expose Guest hart 0
without routing VM 1's external interrupt to virtual PLIC context 1.

## Explicit device assignment

`VmConfig` now owns a `DeviceBusConfig`. Each MMIO assignment records a
Guest-visible base, a Host backend base, and its size. The single-VM QEMU path
uses `VmConfig::qemu_default()`, but no constructor silently grants VM 0's
backend to other VMs.

The current VirtIO block implementation remains a mediated physical backend:
Hypocaust traps register access and validates/translates DMA, while QEMU
executes the request. PR #43 provides the configuration seam needed to assign
one backend per VM. Full software emulation and IOMMU-governed passthrough are
separate device policies and remain later work.

## Safety checks

Construction rejects empty/overflowing MMIO apertures and overlapping
Guest-visible device ranges. `DeviceBus` continues to own one virtual PLIC per
VM, and all completion/claim routing now uses `GuestHartId`.

## Validation

```console
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
make qemu SMP=2
```

The compatibility test must boot xv6-rust, deliver asynchronous VirtIO block
completions through Guest PLIC context 0, and initialize the file system.
