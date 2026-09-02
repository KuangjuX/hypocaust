# Asynchronous mediated VirtIO block backend

PR #42 (`feature/async-virtio-block`) converts the former fire-and-forget QEMU
VirtIO path into a completion-tracked, VM-owned mediated backend. PR #47
(`feature/iommu-passthrough-policy`) makes this terminology an enforced
architectural boundary rather than treating direct Host MMIO as passthrough.

## Backend boundary

The current Host has one physical QEMU VirtIO block device. Hypocaust retains
QEMU as the asynchronous storage executor but owns the Guest-visible frontend:

```text
Guest queue notify
  -> VM DeviceBus
  -> validate complete vring with GuestMemory
  -> validate every newly available descriptor range
  -> translate Guest DMA addresses to the VM's Host RAM slot
  -> notify the assigned QEMU backend

Host timer boundary
  -> poll the backend-owned used ring
  -> detect newly completed requests
  -> raise source 1 in this VM's VirtualPlic
  -> synchronize the target context output with that vCPU's virtual SEIP
```

The vCPU is not blocked for the duration of an I/O request. The Guest is free
to schedule another process, while its block driver sleeps only the requesting
process until completion. This is the normal VirtIO asynchronous execution
model.

## Checked DMA

Queue PFN programming validates the complete legacy vring allocation through
the VM-owned `GuestMemory` capability. Queue notification walks only newly
published descriptor chains and checks every `(address, length)` range before
the physical backend sees a translated Host address. Invalid ranges, queue
indices, overflows, and cyclic chains fail closed.

The completion poller derives the used-ring address from the already checked
queue layout. It keeps a Host-private used index, separate from the Guest
driver's consumed index, so it detects each completion once without modifying
Guest ownership of the ring.

## Interrupt lifecycle

New used-ring entries raise VM-local PLIC source 1. If the chosen PLIC context
has enabled that source above its threshold, its output becomes the target
vCPU's SEIP pending state. Reading PLIC claim removes the source from pending
state; acknowledging the VirtIO interrupt status lowers the level-triggered
source in this VM only. After every PLIC or device MMIO access, Hypocaust
resynchronizes the current context output so claim and acknowledgement can
deassert SEIP promptly.

The existing xv6-rust SBI example still polls its used ring on virtual timer
ticks, which remains compatible. A follow-up example PR can enable its PLIC
path to consume the new interrupt delivery without polling.

## Observability and validation

Hypocaust counts Guest queue notifications and observed completions. It emits
tracking records only when the completion total reaches a power of two, which
proves forward progress without flooding the serial console.

Validation uses:

```console
git diff --check
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
make qemu SMP=2
```

Expected output includes `async VirtIO notifications=... completions=...`, and
xv6-rust must still complete file-system initialization.

## Production evolution

This mediated backend prevents unchecked DMA and virtualizes completion
routing. PR #45 gives each VM an independent QEMU disk backend. PR #47 defines
the separate hardware passthrough manager; general-purpose Guests should keep
using a fully emulated, mediated, or paravirtual backend unless a board adapter
can provide exclusive IOMMU and interrupt-remapping protection.
