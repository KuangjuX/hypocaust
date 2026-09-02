# Per-vCPU virtual interrupt injection

PR #40 (`feature/per-vcpu-virtual-interrupts`) introduces targeted virtual
interrupt state and a Host/device-backend API that identifies its destination
with a complete `VcpuKey`.

## Architectural state

Each vCPU already owns its shadow `sip`, `sie`, `sstatus`, `scause`, `sepc`, and
`stvec` registers. This PR makes those registers the single source of truth
instead of pairing them with a coarse `interrupt: bool` flag.

`VirtualInterrupt` names the supported Supervisor interrupt classes:

- external (`SEIP`, cause 9);
- software (`SSIP`, cause 1);
- timer (`STIP`, cause 5).

Injection sets only the corresponding pending bit in the selected vCPU.
Deassertion clears only that source, leaving unrelated pending interrupts
intact. Deliverable interrupts are masked by that vCPU's `sie`, checked against
its virtual global `sstatus.SIE`, and selected in the RISC-V architectural
priority order SEI, SSI, STI documented by the
[RISC-V Supervisor-Level ISA](https://docs.riscv.org/reference/isa/priv/supervisor.html).

## Injection and wakeup

The Host-facing path is:

```text
device or Host event
  -> inject_virtual_interrupt(VcpuKey, VirtualInterrupt)
  -> set only the target vCPU's shadow sip bit
  -> inspect scheduler state
       Running(hart): send that hart an SBI IPI
       Blocked: make Ready and IPI an online idle hart when available
       Ready: leave its existing run-queue entry unchanged
```

All state changes happen under the Hypervisor lock. The physical IPI is sent
after releasing the lock so the destination can immediately enter its trap
path. Host software interrupts remain enabled while Guest code executes.

An SBI IPI is a Host scheduling event, not a Guest software interrupt. The
handler clears physical SSIP and preserves the current scheduling assignment.
The selected vCPU's virtual pending state is arbitrated immediately before
every Guest entry.

Performing arbitration after scheduling closes an important race: a timer may
preempt the originally targeted vCPU after the sender observes it as running
but before the IPI arrives. The IPI can safely trap the newly selected vCPU;
the original pending bit stays with its owner and is delivered when that vCPU
next runs.

## Exceptions are different

Synchronous Guest exceptions are never injected or broadcast. The trap path
writes `scause`, `stval`, and `sepc` only in the vCPU that executed the faulting
instruction, performs the virtual SIE-to-SPIE transition, and enters that
vCPU's virtual `stvec`.

## Validation

Startup self-tests cover interrupt priority, per-source deassertion, selecting
a running vCPU's Host hart, and waking a blocked vCPU onto an online idle hart.
The xv6-rust boot exercises repeated per-vCPU timer injection and Guest-entry
arbitration:

```console
git diff --check
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
make qemu SMP=2
```

xv6-rust reaches VirtIO-backed file-system initialization while the second
Host hart remains online in the scheduler.

## Next step

The current block backend remains synchronous passthrough. The asynchronous
VirtIO backend PR will use this API to inject an external interrupt into the
configured vCPU after a checked DMA request completes, waking it if necessary.
