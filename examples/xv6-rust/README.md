# xv6-rust Guest example

This example boots [xv6-rust](https://github.com/Ko-oK-OS/xv6-rust) as a
virtual RISC-V S-mode Guest on Hypocaust. xv6-rust remains an independent
project: this directory contains no vendored source and no Git submodule.

## Compatibility baseline

| Component | Validated value |
| --- | --- |
| xv6-rust revision | `b07f26f` |
| xv6-rust branch | `fix-bug/sbi-virtio-completion` |
| Guest entry mode | single-hart SBI payload |
| Guest kernel input | `kernel/target/riscv64gc-unknown-none-elf/debug/kernel` |
| Guest disk input | `fs.img` |

The VirtIO completion fix on the validated branch is required because the
current Guest external-interrupt path is incomplete. In SBI mode, xv6-rust
polls completion while Hypocaust mediates the block device.

## Prepare the Guest checkout

From the directory that contains the Hypocaust checkout:

```sh
git clone --recurse-submodules \
  --branch fix-bug/sbi-virtio-completion \
  https://github.com/Ko-oK-OS/xv6-rust.git
git -C xv6-rust checkout b07f26f
git -C xv6-rust submodule update --init --recursive
```

See the root [prerequisites](../../README.md#prerequisites) before building.

## Run the example

From the Hypocaust repository root:

```sh
make -C examples/xv6-rust run
```

The example builds the xv6-rust filesystem and SBI kernel, copies their output
to Hypocaust's ignored `fs.img` and `guest_kernel` paths, embeds the Guest ELF,
and launches QEMU. It never fetches, checks out, cleans, or edits the external
xv6-rust working tree.

For a checkout in another location:

```sh
make -C examples/xv6-rust run \
  XV6_RUST_DIR=/absolute/path/to/xv6-rust
```

At the shell prompt, run a smoke test:

```text
xv6 Rust >>> echo HYPOCAUST_OK
HYPOCAUST_OK
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
