# Hypocaust
**Hypocaust** is an S-mode trap and emulate type-1 hypervisor for RISC-V. It is currently targeted at QEMU's virt machine type. It can run on [RustSBI](https://github.com/rustsbi/rustsbi) currently.  

![](docs/images/hypocaust.png)

## Environment
- QEMU 7.0.0
- rust 1.66.0

## Memory Region
- DRAM Memory Region: 0x80000000 - 0x140000000 3GB   
- hypervisor: 128MB  
- Guest Kernel: 128MB 

### Hypervisor Memory Region
| HVA Start | HVA End | HPA Start | HPA End | Memory Region |
| --------------| ----------- | -------------- | ------------ | -------------  |
| 0x80000000    | 0x80200000  | 0x80000000     | 0x80200000   |RustSBI        |
| 0x80200000    | 0xC0000000  | 0x80200000     | 0x88000000   |hypervisor     |
| 0x88000000    | 0x8FFFFFFF  | 0x88000000 | 0x8FFFFFFF | Guest Kernel 1   |
| 0x90000000    | 0x97FFFFFFF  | 0x90000000 | 0x97FFFFFF | Guest Kernel 2   |
| 0x98000000    | 0x9FFFFFFFF  | 0x98000000 | 0x9FFFFFFF | Guest Kernel 3   |

### Resvered Memory Region
| VA Start | VA End | Memory Region |
| ---------|--------| -------------- |
| 0xFFFFFFFFFFFFF000 | 0xFFFFFFFFFFFFFFFF | Trampoline |
| 0xFFFFFFFFFFFFE000 | 0xFFFFFFFFFFFFEFFF | Trap Context |

### Guest Kernel Memory Region
| GVA | GPA | HVA | Memory Region |  
| ---- | ---- | ---- | ---- |  
| 0x80000000 - 0x87FFFFFF | 0x80000000 - 0x87FFFFFF | 0x88000000 - 0x8FFFFFFF | Guest Kernel 1 | 
| 0x80000000 - 0x87FFFFFF | 0x80000000 - 0x87FFFFFF | 0x90000000 - 0x97FFFFFF | Guest Kernel 2|
| 0x80000000 - 0x87FFFFFF | 0x80000000 - 0x87FFFFFF | 0x98000000 - 0x9FFFFFFF | Guest Kernel 3 |

![](docs/images/layout.png)

## Supported Platforms
- QEMU virt machine type

## RoadMap
- [x] Load guest kernel && Run guest kernel
- [x] Trap and emulate of privileged instructions(CSR related and SFENCE>VMA)
- [x] Shadow page tables
- [x] Synchronize guest page table & shadow page table
- [x] Foward Expections & Interrupts
- [ ] Update PTE accessed and dirty bits
- [ ] Timers
- [ ] Expose and/or emulate peripherals
- [ ] passthrough virtio block and networkd devices
- [ ] multicore supported
- [ ] multiguest supported

## References
- [rcore-os/rCore-Tutorial-v3](https://github.com/rcore-os/rCore-Tutorial-v3)
- [mit-pdos/RVirt](https://github.com/mit-pdos/RVirt)
