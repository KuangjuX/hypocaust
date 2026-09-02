# Per-VM virtual PLIC

PR #41 (`feature/per-vm-virtual-plic`) adds a virtual Platform-Level Interrupt
Controller to every VM's `DeviceBus`.

## Why SEIP alone is insufficient

PR #40 can set a target vCPU's virtual `sip.SEIP`, but a normal Guest external
interrupt handler then asks its interrupt controller which device interrupted.
It reads a claim register, services that source, and writes the source ID back
to complete it. Without a virtual interrupt controller, injecting SEIP sends
the Guest into another unmapped MMIO fault.

Physical PLIC registers also cannot be shared safely between Guests. Their
pending, enable, priority, threshold, and in-service state must be virtualized
inside each VM.

## Implemented register model

The VM-owned `VirtualPlic` implements the 32-bit PLIC subset used by xv6-rust:

- per-source priority registers;
- one pending-bit word for sources 1 through 31;
- per-context enable words;
- per-context priority thresholds;
- claim reads and completion writes.

Source 0 remains reserved. A claim selects the enabled pending source whose
priority is strictly greater than the context threshold. Higher priority wins;
equal priorities select the lower source ID. Claim atomically removes the
source from pending state and records it as in service. Completion is accepted
only from the context that claimed that source.

The initial supported contexts match Hypocaust's bounded Host/vCPU execution
capacity. The multi-Guest configuration PR will give Guest-local hart indices
an explicit mapping to globally unique `VcpuId` values.

## Device-to-vCPU route

The controller exposes three Host-side operations:

```text
raise(source, context) -> context output asserted?
lower(source)
has_interrupt(context)
```

A device backend raises a source in the owning VM's PLIC. If the selected
context output is asserted, it uses PR #40's targeted injection API to set SEIP
on the vCPU mapped to that context. Claim, device acknowledgement, priority,
enable, and threshold changes can re-evaluate `has_interrupt` and update that
same vCPU's SEIP level.

PLIC MMIO is routed through the per-VM `DeviceBus`; the Host PLIC is never
mapped into the Guest.

## Validation

The startup self-test programs VirtIO source 1, enables it on context 0, raises
it, checks the pending word, claims it, verifies pending is cleared, and
completes it. Build and runtime validation use:

```console
git diff --check
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
make qemu SMP=2
```

The current xv6-rust SBI build still polls block completion and therefore does
not program the PLIC yet. It continues to boot through file-system
initialization, while the following asynchronous VirtIO PR will connect source
1 to targeted SEIP delivery.
