# Hypocaust
**Hypocaust** is a type-1 hypervisor run on RISC-V machine designed for final graduation project.

## Environment
- QEMU 7.0,0
- rust 1.66.0

## Memory Region
| Virtual Start | Virtual End | Physical Start | Physical End | Memory Region |
| --------------| ----------- | -------------- | ------------ | -------------  |
| 0x80000000    | 0x80200000  | 0x80000000     | 0x80200000   |RustSBI        |
| 0x80200000    | 0xC0000000  | 0x80200000     | 0x88000000   |hypervisor     |
| 0xFFFFFFFF88000000    | 0xFFFFFFFF8FFFFFFF  | 0x88000000 | 0x8FFFFFFF | Guest Kernel 1   |
| 0xFFFFFFFF90000000    | 0xFFFFFFF97FFFFFFF  | 0x90000000 | 0x97F00000 | Guest Kernel 2   |

Guest Virtual Address -> Guest Physical Address(Host Virtual Address) -> Host Physical Address

## References
- [rcore-os/rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3)
- [mit-pdos/RVirt](https://github.com/mit-pdos/RVirt)
