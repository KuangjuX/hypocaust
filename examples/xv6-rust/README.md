# xv6-rust Guest example

This example boots [xv6-rust](https://github.com/Ko-oK-OS/xv6-rust) as a
virtual RISC-V S-mode Guest on Hypocaust. xv6-rust remains an independent
project: this directory contains no vendored source and no Git submodule.

## Compatibility baseline

| Component | Validated value |
| --- | --- |
| xv6-rust revision | `0e61a5e` |
| xv6-rust branch | `main` (includes PRs #60 and #61) |
| Guest entry mode | single-hart SBI payload |
| Guest kernel input | `kernel/target/riscv64gc-unknown-none-elf/debug/kernel` |
| Guest disk input | `fs.img` |

PR #60 supplies SBI-mode completion polling and PR #61 connects completion to
Hypocaust's per-VM virtual PLIC. Both are merged into the validated main
revision while Hypocaust mediates the block device.

## Prepare the Guest checkout

From the directory that contains the Hypocaust checkout:

```sh
git clone --recurse-submodules \
  https://github.com/Ko-oK-OS/xv6-rust.git
git -C xv6-rust checkout 0e61a5e
git -C xv6-rust submodule update --init --recursive
```

See the root [prerequisites](../../README.md#prerequisites) before building.

## Run the example

From the Hypocaust repository root:

```sh
make -C examples/xv6-rust run
```

The example builds the xv6-rust filesystem and SBI kernel, copies their output
to Hypocaust's ignored `guest_kernel`, `fs-vm0.img`, and `fs-vm1.img` paths,
embeds the Guest ELF, and launches two VMs on two Host harts. It never fetches,
checks out, cleans, or edits the external xv6-rust working tree.

For a checkout in another location:

```sh
make -C examples/xv6-rust run \
  XV6_RUST_DIR=/absolute/path/to/xv6-rust
```

VM 0 owns initial keyboard focus. At its labelled shell prompt, run:

```text
[Guest VM 0] xv6 Rust >>> echo HYPOCAUST_OK
[Guest VM 0] HYPOCAUST_OK
```

Press `Ctrl-a`, then `x`, to stop QEMU.

## Artifact-only workflow

To refresh the copied Guest artifacts without booting:

```sh
make -C examples/xv6-rust prepare
```

Subsequent `make qemu` commands at the repository root reuse those copies.
`make clean` removes the copies and Hypocaust outputs, but deliberately leaves
the xv6-rust checkout and its build cache untouched.
