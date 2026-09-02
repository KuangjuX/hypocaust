# Exception handling

This compatibility page replaces the original placeholder. The implemented
multi-Guest exception model, routing table, shadow CSR entry semantics, and
Host-fatal boundary are documented in
[Guest exception forwarding](guest-exception-forwarding.md).

In short, synchronous exceptions caused by Guest execution belong to the
current vCPU. Hypocaust emulates recognized privileged instructions, MMIO, and
tracked PTE writes; all other synchronous causes are injected into that vCPU's
virtual S-mode trap state. Exceptions taken while Hypocaust executes and
unexpected physical interrupts remain Host failures.
