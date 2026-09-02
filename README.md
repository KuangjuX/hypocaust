# Hypocaust

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform: RISC-V RV64](https://img.shields.io/badge/platform-RISC--V%20RV64-283272.svg)](#supported-platform)
[![Guest: xv6-rust](https://img.shields.io/badge/guest-xv6--rust-b7410e.svg)](#run-the-xv6-rust-example)

Hypocaust is an experimental type-1 hypervisor that virtualizes a RISC-V
Supervisor-mode guest without requiring the Hypervisor extension. It runs in
S-mode beneath OpenSBI, traps guest privileged operations, and presents
xv6-rust with a virtual supervisor environment backed by shadow page tables.

> [!IMPORTANT]
> Hypocaust is a research and learning project, not a security boundary for
> untrusted workloads. The currently validated configuration is one xv6-rust
> guest on one QEMU `virt` hart.

![Hypocaust architecture](docs/images/hypocaust.png)

## What works

- Guest S-mode CSR and `sfence.vma` trap-and-emulate
- Sv39 guest-to-shadow page-table synchronization
- Write trapping for live guest page-table pages
- Cached shadow roots with generation-based resynchronization
- ASID-tagged shadow translations and targeted TLB invalidation
- SBI console and timer services
- VirtIO block access used by the xv6-rust filesystem
- Runtime shadow-paging profiling counters

Current limitations include single-hart/single-guest operation, QEMU-only
validation, incomplete device virtualization, and no hardened isolation model.

## Architecture

Hypocaust is the Host S-mode payload loaded by QEMU's bundled OpenSBI. The
xv6-rust kernel is embedded as an ELF image and starts as the virtual Guest
S-mode payload at guest physical address `0x8000_0000`.

```text
xv6-rust user programs (Guest U-mode)
                 |
xv6-rust kernel (virtual Guest S-mode)
                 |  privileged operations, PTE writes, SBI calls
                 v
Hypocaust (Host S-mode): trap emulation + shadow Sv39 + device mediation
                 |
OpenSBI (M-mode) + QEMU virt machine
```

For a Guest virtual address, xv6-rust's Sv39 tree first defines the Guest
virtual-to-physical mapping. Hypocaust mirrors that state into a hardware-walked
shadow Sv39 tree whose leaves point at Host physical memory. Guest PTE pages are
write-protected so changes trap and can be reflected before execution resumes.

The performance work is documented in:

- [shadow-paging profiling](docs/shadow-paging-profile.md)
- [synchronization generation cache](docs/shadow-page-table-cache.md)
- [shadow page-table ASIDs](docs/shadow-page-table-asid.md)
- [incremental valid-PTE counts](docs/valid-pte-count.md)

## Supported platform

| Component | Validated configuration |
| --- | --- |
| ISA | RV64GC with Sv39; the RISC-V H extension is not required |
| Machine | QEMU `virt` |
| Firmware | QEMU bundled OpenSBI (`-bios default`) |
| vCPUs | 1 |
| RAM | 3 GiB |
| Guest | xv6-rust SBI payload |
| xv6-rust revision | `b07f26f` (`fix-bug/sbi-virtio-completion`) |
| QEMU | 11.1.1 |
| Rust | `nightly-2026-09-02` (pinned by `rust-toolchain.toml`) |

Other QEMU or toolchain versions may work but are not part of the current
compatibility baseline.

## Prerequisites

Install these tools and make sure they are on `PATH`:

- Git and Make
- Rust through [rustup](https://rustup.rs/)
- `qemu-system-riscv64`
- a RISC-V bare-metal C toolchain providing `riscv64-unknown-elf-*` or
  `riscv64-elf-*` (used to build xv6-rust userspace)
- a host C compiler, Perl, and Python 3 (used by xv6-rust)

`rust-toolchain.toml` installs the pinned compiler, RISC-V target, and LLVM
tools component automatically. Install the command wrappers used by `make asm`
and the binary target if needed:

```sh
cargo install cargo-binutils --version 0.4.0 --locked
```

Verify the two runtime dependencies:

```sh
rustc --version
qemu-system-riscv64 --version
```

## Run the xv6-rust example

Clone Hypocaust and the compatible xv6-rust branch as sibling directories. The
SBI VirtIO completion fix on this branch is required for filesystem I/O under
Hypocaust and is the configuration used by the end-to-end validation.

```sh
git clone https://github.com/KuangjuX/hypocaust.git
git clone --recurse-submodules \
  --branch fix-bug/sbi-virtio-completion \
  https://github.com/Ko-oK-OS/xv6-rust.git
git -C xv6-rust checkout b07f26f
git -C xv6-rust submodule update --init --recursive
cd hypocaust
make -C examples/xv6-rust run
```

The example performs four explicit steps:

1. builds `fs.img` in the xv6-rust checkout;
2. builds xv6-rust's single-hart `sbi` kernel configuration;
3. copies the kernel ELF to `guest_kernel` and the disk image to `fs.img`;
4. embeds the kernel in Hypocaust and boots QEMU.

If xv6-rust is elsewhere, override its location:

```sh
make -C examples/xv6-rust run \
  XV6_RUST_DIR=/absolute/path/to/xv6-rust
```

A successful boot ends at the guest prompt. For a smoke test:

```text
xv6 Rust >>> echo HYPOCAUST_OK
HYPOCAUST_OK
```

Press `Ctrl-a`, then `x`, to exit QEMU. After guest artifacts have been
prepared once, `make qemu` reuses the local copies without rebuilding xv6-rust.
Run `make xv6-rust` whenever the guest checkout changes.

The explicit xv6-rust commit above is the reproducible compatibility baseline.
To test newer changes, switch that independent checkout to the desired revision
and rerun `make -C examples/xv6-rust run`; Hypocaust never changes its branch
or working tree.
The complete integration contract and artifact workflow live in the
[xv6-rust Guest example](examples/xv6-rust/).

## Build and development commands

| Command | Purpose |
| --- | --- |
| `make help` | Show the supported workflow and path override |
| `make xv6-rust` | Build and copy xv6-rust kernel/filesystem artifacts |
| `make build` | Build Hypocaust with existing guest artifacts |
| `make qemu` | Boot the existing guest artifacts |
| `make qemu-xv6` | Refresh xv6-rust artifacts and boot them |
| `make qemu-gdb` | Start QEMU paused with a GDB server on port 1234 |
| `make asm` | Generate Host and Guest disassemblies |
| `make clean` | Remove Hypocaust outputs and copied guest artifacts |

The guest artifacts are ignored by Git. `make clean` does not modify or clean
the independent xv6-rust checkout.

### Debug with GDB

In one terminal:

```sh
make qemu-gdb
```

In another terminal:

```sh
gdb-multiarch target/riscv64gc-unknown-none-elf/debug/hypocaust
(gdb) set architecture riscv:rv64
(gdb) target remote :1234
```

`make debug` provides the same setup in a tmux split when both tools are
installed.

## Memory layout

| Region | Host physical range | Purpose |
| --- | --- | --- |
| Firmware | `0x8000_0000..0x8020_0000` | OpenSBI |
| Hypocaust | `0x8020_0000..0x8800_0000` | Host code, data, heap, and page tables |
| Guest 0 | `0x8800_0000..0x9000_0000` | 128 MiB Guest RAM backing |
| Guest 1 reserved | `0x9000_0000..0x9800_0000` | Future Guest RAM backing |
| Guest 2 reserved | `0x9800_0000..0xa000_0000` | Future Guest RAM backing |
| Shadow tables | from `0x1_0000_0000` | Guest 0 shadow page-table arena |

The active guest sees RAM at `0x8000_0000..0x8800_0000`. Host trampoline and
trap-context pages occupy the top two canonical virtual pages. See
[the layout diagram](docs/images/layout.png) for the address translation view.

## Repository layout

| Path | Purpose |
| --- | --- |
| `src/hypervisor/` | trap entry, SBI/CSR emulation, faults, and Host runtime |
| `src/guest/` | Guest state and shadow page-table lifecycle |
| `src/device_emu/` | emulated or mediated device access |
| `src/mm/` | Host memory sets and Guest ELF loading |
| `src/page_table/` | Sv39 page-table implementation |
| `src/constants/` | board and memory-layout constants |
| `docs/` | architecture, profiling, and design notes |
| `examples/xv6-rust/` | external xv6-rust Guest integration example |

## Troubleshooting

**`xv6-rust was not found`** — place the checkout at `../xv6-rust` or pass
`XV6_RUST_DIR=/absolute/path/to/xv6-rust`.

**RISC-V compiler not found** — install a bare-metal GCC toolchain and confirm
that either `riscv64-unknown-elf-gcc` or `riscv64-elf-gcc` is on `PATH`.

**Guest boots but filesystem operations stall** — use the documented
`fix-bug/sbi-virtio-completion` xv6-rust branch; its SBI-mode driver polls
VirtIO completion because the current virtual interrupt path is incomplete.

**Port 1234 is busy** — stop the other debugger session or change the port in
both the QEMU and GDB commands.

## Contributing

Keep changes focused and submit each feature or bug fix independently. Branch
names use `feature/<description>` or `fix-bug/<description>`. Pull requests
should explain the problem, design and invariants, testing evidence, performance
impact where relevant, and follow-up limitations. Non-obvious code must include
a comment describing its purpose and the PR that introduced it.

Before opening a pull request, run:

```sh
cargo build --features embed_guest_kernel
cargo build --release --features embed_guest_kernel
git diff --check
```

Changes to virtualization behavior should additionally boot xv6-rust and run a
guest command that exercises the affected path.

## Related work

- [hypocaust-2](https://github.com/KuangjuX/hypocaust-2), the hardware-assisted
  virtualization successor
- [xv6-rust](https://github.com/Ko-oK-OS/xv6-rust), the validated Guest OS
- [RVirt](https://github.com/mit-pdos/RVirt)
- [rCore Tutorial](https://github.com/rcore-os/rCore-Tutorial-v3)

## License

Hypocaust is available under the [MIT License](LICENSE).
