# Per-VM console frontend

PR #49 (`feature/per-vm-console`) gives every VM its own legacy SBI console
frontend while multiplexing output onto the single Host firmware console.

## Previous behavior

Every Guest `SBI_CONSOLE_PUTCHAR` called the Host SBI console directly. A lock
protected Hypervisor state only for one trap, so another vCPU could emit its
next character between any two bytes. With multiple Guests, boot logs and
shell prompts were impossible to attribute reliably.

`SBI_CONSOLE_GETCHAR` was also unrestricted. Whichever Guest happened to poll
first could consume a physical console byte intended for another VM.

## Output records

The VM-owned `Uart` frontend now buffers console bytes until newline or until
its fixed 256-byte buffer fills. It then emits one labelled Host record:

```text
[Guest VM 0] xv6-rust kernel is booting!
[Guest VM 1] xv6-rust kernel is booting!
```

Partial prompts are flushed before an input poll so an interactive user can
see them. Guest lines and Hypocaust log records use one Host output lock, which
prevents other Host harts from splicing their records into a Guest line. The
buffer is fixed-capacity and performs no allocation in trap context.

## Input focus

The physical firmware console is a singleton, so it cannot safely be presented
as independent input to every VM. An atomic Host-side focus selects exactly one
`VmId`; other Guests receive the legacy SBI "no character available" value.
VM 0 owns the initial focus. `Uart::set_console_focus` is the management-plane
hook for a future monitor or control protocol to switch it.

Production systems commonly attach each VM to a separate PTY, socket, or
virtio-console backend instead. The focus mechanism is the safe minimum for a
single QEMU serial terminal and does not claim multiple independent physical
input streams.

## Ownership path

```text
Guest legacy SBI ecall
  -> current VcpuKey
  -> owning VirtualMachine
  -> that VM's DeviceBus
  -> that VM's Uart buffer/input policy
  -> serialized Host firmware console
```

The privileged-instruction handler now receives the current `DeviceBus` so it
cannot bypass VM ownership when servicing a console SBI call.

## Firmware diagnostic limitation

The QEMU boot path currently deprivileges Guest S-mode into physical U-mode.
Privileged instructions first reach OpenSBI in M-mode, whose diagnostic build
prints `system_opcode_insn: Invalid opcode ...` messages. Those writes happen
outside Hypocaust and therefore outside its output lock; they can still appear
inside a labelled partial prompt. Removing that noise requires a compatible
OpenSBI build or RISC-V H-extension execution and is not hidden by this feature.

## Validation

The startup self-test checks line buffering and input-focus switching without
reading or writing the physical console. Debug and release builds succeed.
With `make qemu SMP=2`, both xv6-rust Guests emit independently labelled boot
records, initialize their filesystems, and reach VirtIO completion counts 1,
2, 4, and 8 without a Hypocaust or Guest panic.
