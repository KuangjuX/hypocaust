//! Constants for QEMU's RISC-V `virt` machine.

/// Frequency of the `virt` machine's ACLINT MTIMER and architectural `time` CSR.
///
/// PR #53 (`fix-bug/qemu-timebase-frequency`) replaces the inherited rCore
/// value with the 10 MHz rate advertised by QEMU/OpenSBI. The same constant
/// drives Host timer deadlines and the per-VM Linux device tree, so keeping it
/// accurate prevents both scheduler ticks and Guest wall-clock time from being
/// scaled by the previous 25% error.
pub const CLOCK_FREQ: usize = 10_000_000;

pub const MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
    (0x1000_1000, 0x00_1000), // Virtio Block in virt machine
];
