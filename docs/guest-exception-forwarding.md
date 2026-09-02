# Guest exception forwarding and containment

PR #48 (`fix-bug/guest-exception-forwarding`) makes synchronous exceptions a
vCPU-local architectural event instead of a Hypervisor-wide panic condition.

## Trap ownership

Hypocaust now classifies traps before handling them:

| Trap class | Owner | Action |
| --- | --- | --- |
| Guest ecall or illegal instruction | current vCPU | emulate a supported SBI/privileged operation, otherwise inject the original exception |
| Guest load/store page fault | current vCPU | emulate known MMIO or a tracked PTE write, otherwise inject the original exception |
| Any other synchronous Guest exception | current vCPU | inject through the Guest's virtual S-mode trap state |
| Host timer interrupt | Host scheduler | poll devices, update the current vCPU timer, then preempt |
| Host software interrupt | Host scheduler | acknowledge the scheduling/IPI event |
| Unexpected physical interrupt | Host | fail as a Host integration error; never disguise it as a Guest exception |

Forwarding stores `scause`, `sepc`, and `stval` in the selected vCPU's shadow
CSRs, performs the virtual `SIE -> SPIE` transition, records the interrupted
virtual privilege in `SPP`, and jumps to that Guest's `stvec`. No other VM or
vCPU observes those registers.

## Fixed failure modes

Previously, several Guest-controlled cases could panic all of Hypocaust:

- a breakpoint was sent to the privileged-instruction decoder;
- an unrecognized instruction or CSR reached `panic!`, `unreachable!`, or an
  address-translation `unwrap`;
- address-selective `SFENCE.VMA` reached `unimplemented!`;
- an ordinary, unaligned Guest page fault hit a Host PTE-alignment assertion;
- unsupported MMIO access widths panicked instead of preserving the page fault;
- a malformed non-leaf Guest PTE could request shadow memory outside its VM.

An undecodable instruction also had a control-flow bug: Hypocaust installed
the Guest trap vector and then incremented `ctx.sepc` by two or four, entering
the handler at `stvec + instruction_length`. Forwarding paths now return
immediately and preserve the exact trap-vector address.

## Page-table write recognition

Alignment alone does not prove that a faulting store is a page-table update.
PR #48 checks all of the following before incremental shadow synchronization:

1. the fault is not Guest U-mode or bare-address mode;
2. the address is PTE aligned;
3. the instruction is a decoded 64-bit store whose effective address matches
   `stval`;
4. the GVA resolves through the owning VM's mappings;
5. the resulting HPA converts back into that VM's canonical GPA;
6. the page is recorded as a Guest page-table page.

Newly linked non-leaf pages are recorded immediately. Out-of-RAM non-leaves
become invalid shadow entries, allowing the Guest to receive its architectural
page fault without dereferencing another VM or Host address.

## TLB fences

Both global and address-selective Guest `SFENCE.VMA` instructions are accepted.
PTE writes are synchronized eagerly, while the fence conservatively marks all
cached shadow ASIDs dirty. The existing return path issues an ASID-scoped Host
`SFENCE.VMA` before the affected shadow root runs again.

## Validation

The startup routing self-test checks breakpoint forwarding, illegal-instruction
emulation selection, and Host timer routing. Debug and release builds succeed.
The `SMP=2` QEMU regression keeps both xv6-rust Guests running on independent
VirtIO backends through asynchronous completion counts 1, 2, 4, and 8 without
a Hypocaust or Guest panic.

## Remaining Host-fatal conditions

An unexpected physical interrupt and an exception taken while Hypocaust itself
is executing remain Host-fatal. These indicate a board/driver or Hypervisor
bug, not untrusted Guest architectural input. A later VM lifecycle feature can
add explicit VM termination for policy violations that cannot be represented
as a RISC-V Guest exception.
