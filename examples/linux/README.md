# Alpine Linux Guest example

This example boots two Alpine Linux 3.24.1 RISC-V Guests to independent
initramfs BusyBox shells. Linux remains an external build artifact: this
directory contains neither vendored Linux source nor a Git submodule.

## Artifact and boot contract

| Item | Validated value |
| --- | --- |
| Distribution | Alpine Linux 3.24.1 `riscv64` |
| Source | official `alpine-standard-3.24.1-riscv64.iso` |
| Kernel | Linux 6.18.35 `vmlinuz-lts`, decompressed to `Image` |
| Initramfs | Alpine `initramfs-lts` |
| Init process | `/bin/sh` through `rdinit=/bin/sh` |
| Console | per-VM virtual NS16550A at `0x1000_0000` |

PR #73 (`feature/linux-initramfs-shell-example`) pins the release instead of
following a moving `latest` URL. The Makefile downloads the ISO and its
published SHA-256 file, verifies it, and extracts only the kernel and initramfs.

## Prerequisites

Install the root project prerequisites plus:

- `curl`;
- `gzip`;
- `bsdtar` from libarchive;
- `python3`;
- either `sha256sum` or `shasum`.

## Run

From the repository root:

```sh
make qemu-linux
```

The workflow places generated files under the ignored `target/` directory,
installs ignored `guest_kernel` and `linux_initrd` build inputs, creates one
empty block image per VM, and starts QEMU with two Host harts.

A successful run contains both:

```text
[Guest VM 0] Run /bin/sh as init process
[Guest VM 1] Run /bin/sh as init process
```

The shells warn that job control is unavailable because `/bin/sh` is PID 1
without a controlling terminal. This does not prevent command execution. VM 0
owns keyboard focus and displays a labelled `~ #` prompt. Use BusyBox applets
through their explicit path, for example:

```text
~ # /bin/busybox echo HYPOCAUST_LINUX_OK
HYPOCAUST_LINUX_OK
```

Press `Ctrl-a`, then `x`, to stop QEMU.

## Firmware delegation

PR #74 prepares a private copy of QEMU's OpenSBI v1.8.1 firmware and adds the
illegal-instruction bit to its `medeleg` mask. Deprivileged Guest supervisor
instructions then trap directly from U-mode to Hypocaust in Host S-mode instead
of first entering M-mode and emitting an OpenSBI diagnostic. The patcher checks
for exactly one known instruction sequence and fails closed on other firmware
layouts. Set `QEMU_OPENSBI` explicitly when QEMU cannot report its data path.

## Cleanup

```sh
make -C examples/linux clean
```

This removes only downloaded/generated Linux example artifacts. It does not
modify the repository history or any external Guest checkout.
