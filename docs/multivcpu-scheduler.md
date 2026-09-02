# Multi-vCPU scheduler

PR #38 (`feature/multivcpu-scheduler`) replaces Hypocaust's single global
current-vCPU selection with a scheduler that can run different vCPUs on
different Host harts.

## Execution model

Each registered vCPU is identified by a `VcpuKey` containing both its owning
`VmId` and globally unique `VcpuId`. The scheduler owns three explicit states:

- `Ready`: queued and eligible to run;
- `Running(HartId)`: exclusively assigned to one Host hart;
- `Blocked`: not runnable until an emulated device or another Host event wakes
  it.

The scheduler also owns one current-vCPU slot per supported Host hart. All
state transitions happen while the global Hypervisor lock is held, so the same
vCPU cannot be selected concurrently by two harts. The lock is released before
entering Guest code.

Host timer interrupts return the current vCPU to the round-robin queue and
select the next ready vCPU. A blocked vCPU can be made ready with `wake_vcpu`.
If an online Host hart is idle, the wake path sends that hart an SBI IPI; its
Host software-interrupt handler clears SSIP and returns to the scheduler loop.
Offline harts are never selected as IPI targets.

## Hart and trap isolation

The boot hart starts secondary harts with the SBI HSM extension after shared
Host mappings and VM state are initialized. Each secondary hart installs the
Host page table and trap vector before marking itself online.

Hypocaust reserves Host register `tp` for `HartId`. Guest `tp` is now saved in
the per-vCPU `TrapContext` before trap entry restores the Host identity. The
trap context records the Host hart currently running that vCPU, which lets the
common trap path select the correct per-hart scheduler slot.

The old unsafe `force_unlock` calls are removed. Both first entry and every
trap return acquire the Hypervisor lock only to select state, then release it
before switching address spaces or entering Guest code.

## Validation

The bare-metal startup self-test covers registration, scheduling on two harts,
blocking, waking an idle online hart, and timer-style preemption. Runtime
validation used:

```console
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
make qemu
make qemu SMP=2
```

Both QEMU configurations boot xv6-rust through file-system initialization. In
the two-hart configuration, hart 1 reaches its scheduler idle loop while hart 0
runs VM 0 vCPU 0.

## Current boundary

This PR establishes multi-vCPU execution and wakeup mechanics, but the boot
configuration still creates one VM with one vCPU. Later PRs will attach
per-VM device buses, route virtual interrupts to a selected vCPU, and add a
multi-Guest configuration that exercises concurrent vCPU execution.
