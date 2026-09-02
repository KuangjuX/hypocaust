# Per-hart early boot stacks

PR #37 (`fix-bug/per-hart-boot-stacks`) prevents secondary Host harts from
computing stack pointers outside Hypocaust's boot-stack allocation.

The previous entry code allocated one 64 KiB array, then multiplied that full
array size by `hart_id + 1`. Hart 0 landed at the end of the array, but hart 1
landed 64 KiB beyond it. Any firmware-started secondary hart would therefore
enter Rust with an invalid stack and corrupt unrelated memory.

Hypocaust now defines an explicit four-hart early-boot limit, reserves one
64 KiB stack for every supported hart, and uses the per-hart stack size as the
assembly stride. A hart ID outside the supported range is rejected in assembly
before a stack address is calculated.

Until the scheduler starts secondary harts, an in-range secondary that enters
from firmware logs its identity and parks with `wfi` on its own stack. The
multi-vCPU scheduler PR replaces that parking path with the Host-hart run loop.
