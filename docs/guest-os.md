# Guest OS integration contract

The validated Guest is the external
[xv6-rust](https://github.com/Ko-oK-OS/xv6-rust) SBI payload at revision
`0e61a5e`. It is an example dependency, not a Hypocaust submodule.

A compatible Guest currently needs:

- an RV64 Sv39 kernel linked to start at Guest physical `0x8000_0000`;
- one boot vCPU with its VM-local hart ID in `a0`;
- a flattened device tree GPA in `a1`;
- legacy SBI console and timer calls supported by Hypocaust;
- the per-VM virtual PLIC and one legacy VirtIO MMIO block frontend described
  by the supplied device tree;
- 64-bit stores for live PTE updates tracked by the current incremental shadow
  page-table implementation.

The Guest sees 128 MiB RAM at `0x8000_0000..0x8800_0000`. DMA addresses are
Guest physical addresses and must remain inside that RAM. The mediated VirtIO
backend performs checked GPA→HPA translation; a Guest must never know or submit
another VM's Host address.

Build and run instructions live in the root
[README](../README.md#run-the-xv6-rust-example) and the
[xv6-rust example](../examples/xv6-rust/README.md).
