# IOMMU-protected device passthrough

PR #47 (`feature/iommu-passthrough-policy`) separates real hardware
passthrough from Hypocaust's mediated QEMU VirtIO backend.

## Why the distinction matters

The QEMU block path traps every Guest MMIO access, translates and validates
legacy VirtIO queues and descriptors, then programs a Host-side QEMU device.
Hypocaust remains in the data-plane contract, so this is a mediated backend.

Real passthrough lets a physical device perform DMA on behalf of one VM. An
MMIO mapping alone is not isolation: without an IOMMU, a malicious or faulty
Guest can ask the device to overwrite Host or another VM's memory. Sharing a
requester, IOMMU domain, or interrupt route also creates cross-VM state.

## Assignment capability

A `PassthroughAssignment` is complete only when it names all of these:

- the owning `VmId`;
- an opaque, platform-defined `PhysicalDeviceId`;
- an `IommuDomainId`;
- separate Guest-visible and Host physical MMIO apertures;
- a DMA aperture expressed only in Guest physical RAM;
- a Host IRQ to virtual PLIC source/context remapping.

The DMA aperture deliberately contains no arbitrary Host address. The
board-specific adapter receives the owning `GuestMemory` and must derive the
Host mapping through its checked GPA-to-HPA translation.

## Ownership rules

`PassthroughManager` rejects an assignment when:

- its declared owner does not match the supplied `GuestMemory`;
- MMIO or DMA arithmetic overflows, either range is empty, or a hardware
  mapping aperture is not page-aligned;
- Guest MMIO overlaps the VM's RAM or virtual PLIC;
- DMA reaches outside the owning VM's RAM;
- the physical device is already assigned;
- a Host MMIO aperture aliases another active assignment;
- Guest MMIO or Guest interrupt sources overlap inside one VM;
- a Host interrupt is already assigned to another device;
- an IOMMU domain is already owned by another VM;
- the interrupt route has no source or addresses an invalid Guest context;
- the fixed active-assignment capacity is exhausted.

Multiple devices may share one domain only when they have the same VM owner.
The same Guest MMIO address may be used by different VMs because each VM has a
private address space; the corresponding Host aperture must remain exclusive.

## Platform adapter contract

Hardware integration implements `IommuPassthroughAdapter`. Its `activate`
operation must be atomic and fail closed:

1. quiesce the device and disable bus mastering;
2. install only the assignment's checked DMA mappings;
3. attach the physical requester to the owning IOMMU domain;
4. install the Host IRQ remapping to the declared VM/source/context;
5. map the device MMIO aperture for the owning VM;
6. enable interrupts and bus mastering only after every prior step succeeds.

If activation fails, no externally observable assignment may remain. During
`deactivate`, the adapter performs the reverse order and stops DMA and IRQs
before removing translations. If deactivation fails, `PassthroughManager`
keeps the assignment active and owned instead of making the device available
to another VM prematurely.

The current QEMU `virt` integration does not expose a RISC-V IOMMU adapter, so
it cannot instantiate this path. It continues using the checked mediated
VirtIO backend. This is an intentional production safety boundary, not a
temporary silent fallback to unprotected passthrough.

## Multi-Guest device model

The resulting device lifecycle is:

```text
firmware discovery
  -> Host reserves the physical device
  -> PassthroughManager validates exclusive ownership
  -> platform adapter creates IOMMU and IRQ mappings
  -> VM receives its MMIO mapping
  -> device completion is injected into that VM's virtual PLIC
  -> teardown quiesces DMA/IRQ before ownership is released
```

Devices without an IOMMU-capable adapter must use a software-emulated or
mediated backend. Shared devices such as consoles and network/storage services
normally expose one virtual frontend per VM and multiplex Host resources in
the hypervisor or a service domain.

## Validation

The bare-metal startup self-test verifies successful assignment and revocation,
duplicate-device rejection, and cross-VM IOMMU-domain rejection. The existing
dual-Guest QEMU regression verifies that keeping the block path mediated still
boots two xv6-rust Guests with independent disks.
