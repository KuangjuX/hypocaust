# VM, vCPU, and hart identities

`feature/vm-vcpu-identities` introduces the first ownership seam required for
running more than one Guest. The previous runtime used one integer as both a
Guest index and a physical hart number. That makes migration and scheduling
unsafe: moving a Guest to another hart can accidentally select different RAM,
shadow page tables, devices, or Host stacks.

## Identity model

- `VmId` identifies the isolation domain. Guest-physical translation is keyed
  by this value, so it remains stable when a vCPU moves between Host harts.
- `VcpuId` identifies one virtual processor and its saved architectural state.
  IDs are globally unique for now because they also select a Host kernel-stack
  slot.
- `HartId` identifies a physical RISC-V hart. It is used only for Host boot and
  will later be used by the scheduler to record where a vCPU is running.

`VirtualMachine` owns its `Vcpu` objects, and `Hypervisor` owns the set of VMs.
Selection is therefore explicit:

```text
Hypervisor
  +-- VM 0
  |    +-- vCPU 0
  |    `-- vCPU 1
  `-- VM 1
       `-- vCPU 2
```

The runtime rejects duplicate VM IDs, vCPUs attached to the wrong VM, and
duplicate global vCPU IDs. These checks prevent two isolation domains from
sharing the same runtime identity or Host stack.

## Trap ownership

The trap path resolves the current `(VmId, VcpuId)` pair and then updates that
vCPU's saved CSRs, privilege mode, shadow-page-table state, and trap context.
Guest exceptions therefore remain local to the vCPU that caused them. A Guest
page-table walk uses `VmId`; it no longer depends on the Host hart number.

## Deliberate limitations

This PR establishes ownership without changing the boot topology: Hypocaust
still starts one VM with one vCPU. Guest RAM and the virtual-device aggregate
remain stored in the vCPU temporarily. Follow-up PRs will move memory and
devices to the VM boundary, add scheduling, and introduce per-vCPU interrupt
injection.

## Validation

The compatibility boundary is the existing xv6-rust example. Both debug and
release Hypocaust builds must succeed, and xv6-rust must reach its shell and
execute a command through the emulated VirtIO block device.
