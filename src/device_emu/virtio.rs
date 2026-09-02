//! refs: https://github.com/mit-pdos/RVirt/blob/HEAD/src/virtio.rs
//!
//! PR fix-bug/virtio-dma-translation: intercept legacy VirtIO MMIO queue
//! setup and notifications, translating guest queue/descriptor addresses to
//! host physical addresses before the passthrough QEMU device performs DMA.

use arrayvec::ArrayVec;
use core::ptr::{read_volatile, write_volatile};

use crate::constants::layout::{GUEST_KERNEL_VIRT_END, GUEST_KERNEL_VIRT_START};
use crate::guest::gpa2hpa;
use crate::mm::MemoryRegion;

pub const MAX_QUEUES: usize = 4;
pub const MAX_DEVICES: usize = 4;

const VIRTIO_MMIO_QUEUE_SEL: usize = 0x030;
const VIRTIO_MMIO_QUEUE_NUM: usize = 0x038;
const VIRTIO_MMIO_QUEUE_PFN: usize = 0x040;
const VIRTIO_MMIO_QUEUE_NOTIFY: usize = 0x050;
const VIRTIO_MMIO_STATUS: usize = 0x070;

const VRING_DESC_F_NEXT: u16 = 1;

pub struct VirtIO {
    pub devices: ArrayVec<Device, MAX_DEVICES>,
}

#[derive(Copy, Clone)]
pub struct Queue {
    /// Address guest thinks queue is mapped at
    guest_pa: usize, 
    /// Address queue is actually mapped at
    host_pa: usize,
    /// Number of entries in queue
    size: usize,
    /// Last available-ring index whose descriptors were translated.
    last_avail_idx: u16,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Descriptor {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

pub enum Device {
    Passthrough {
        /// Virtual Queue Index, offset=0x30
        queue_sel: u32,
        queues: [Queue; MAX_QUEUES],
        device_registers: MemoryRegion<u32>
    },
    Unmapped
}

impl Device {
    pub fn new(host_base_address: usize) -> Self {
        Device::Passthrough { 
            queue_sel: 0,
            queues: [Queue {
                guest_pa: 0,
                host_pa: 0,
                size: 0,
                last_avail_idx: 0,
            }; MAX_QUEUES],
            device_registers: MemoryRegion::new(host_base_address, 0x1000)
        }
    }
}

impl VirtIO {
    pub fn new(host_base_address: usize) -> Self {
        let mut devices = ArrayVec::new();
        devices.push(Device::new(host_base_address));
        Self { devices }
    }

    pub fn read(&self, address: usize) -> u32 {
        let device = self.devices.iter().find(|device| device.contains(address))
            .expect("virtio read outside registered device");
        match device {
            Device::Passthrough { queue_sel, queues, device_registers } => {
                let offset = address - device_registers.base();
                if offset == VIRTIO_MMIO_QUEUE_PFN {
                    assert!((*queue_sel as usize) < queues.len(), "virtio queue selection out of range");
                    return (queues[*queue_sel as usize].guest_pa >> 12) as u32;
                }
                unsafe { read_volatile(address as *const u32) }
            },
            Device::Unmapped => unreachable!(),
        }
    }

    pub fn write(&mut self, address: usize, value: u32, guest_id: usize) {
        let device = self.devices.iter_mut().find(|device| device.contains(address))
            .expect("virtio write outside registered device");
        match device {
            Device::Passthrough { queue_sel, queues, device_registers } => {
                let offset = address - device_registers.base();
                let mut device_value = value;
                match offset {
                    VIRTIO_MMIO_QUEUE_SEL => {
                        assert!((value as usize) < queues.len(), "virtio queue selection out of range");
                        *queue_sel = value;
                    },
                    VIRTIO_MMIO_QUEUE_NUM => {
                        queues[*queue_sel as usize].size = value as usize;
                    },
                    VIRTIO_MMIO_QUEUE_PFN => {
                        // Preserve the guest-visible PFN in software while
                        // programming QEMU with the translated host PFN.
                        let queue = &mut queues[*queue_sel as usize];
                        queue.guest_pa = (value as usize) << 12;
                        assert!(
                            queue.guest_pa == 0 ||
                            (queue.guest_pa >= GUEST_KERNEL_VIRT_START &&
                             queue.guest_pa < GUEST_KERNEL_VIRT_END),
                            "virtio queue address is outside guest RAM",
                        );
                        queue.host_pa = if queue.guest_pa == 0 {
                            0
                        } else {
                            gpa2hpa(queue.guest_pa, guest_id)
                        };
                        queue.last_avail_idx = 0;
                        device_value = (queue.host_pa >> 12) as u32;
                    },
                    VIRTIO_MMIO_QUEUE_NOTIFY => {
                        assert!((value as usize) < queues.len(), "virtio queue notification out of range");
                        let queue = &mut queues[value as usize];
                        Self::translate_available(queue, guest_id);
                    },
                    VIRTIO_MMIO_STATUS if value == 0 => {
                        for queue in queues.iter_mut() {
                            queue.guest_pa = 0;
                            queue.host_pa = 0;
                            queue.size = 0;
                            queue.last_avail_idx = 0;
                        }
                    },
                    _ => {},
                }
                unsafe { write_volatile(address as *mut u32, device_value) };
            },
            Device::Unmapped => unreachable!(),
        }
    }

    fn translate_available(queue: &mut Queue, guest_id: usize) {
        // Translate only newly published chains. Rewalking old entries could
        // translate an already-host address a second time.
        assert!(queue.host_pa != 0 && queue.size != 0, "virtio queue is not configured");
        let avail = queue.host_pa + queue.size * core::mem::size_of::<Descriptor>();
        let avail_idx = unsafe { read_volatile((avail + 2) as *const u16) };
        let mut translated = 0;
        while queue.last_avail_idx != avail_idx {
            assert!(translated < queue.size, "virtio available ring advanced too far");
            let ring_slot = queue.last_avail_idx as usize % queue.size;
            let head = unsafe { read_volatile((avail + 4 + ring_slot * 2) as *const u16) };
            Self::translate_chain(queue, head as usize, guest_id);
            queue.last_avail_idx = queue.last_avail_idx.wrapping_add(1);
            translated += 1;
        }
    }

    fn translate_chain(queue: &Queue, mut index: usize, guest_id: usize) {
        for _ in 0..queue.size {
            assert!(index < queue.size, "virtio descriptor index out of range");
            let descriptor = unsafe {
                &mut *((queue.host_pa + index * core::mem::size_of::<Descriptor>()) as *mut Descriptor)
            };
            let guest_pa = unsafe { read_volatile(&descriptor.addr) } as usize;
            assert!(
                guest_pa >= GUEST_KERNEL_VIRT_START && guest_pa < GUEST_KERNEL_VIRT_END,
                "virtio descriptor address is outside guest RAM",
            );
            unsafe { write_volatile(&mut descriptor.addr, gpa2hpa(guest_pa, guest_id) as u64) };
            let flags = unsafe { read_volatile(&descriptor.flags) };
            if flags & VRING_DESC_F_NEXT == 0 {
                return;
            }
            index = unsafe { read_volatile(&descriptor.next) } as usize;
        }
        panic!("virtio descriptor chain contains a cycle");
    }
}

impl Device {
    fn contains(&self, address: usize) -> bool {
        match self {
            Device::Passthrough { device_registers, .. } => device_registers.in_region(address),
            Device::Unmapped => false,
        }
    }
}

pub fn is_device_access(guest_pa: usize) -> bool {
    guest_pa >= 0x1000_1000 && guest_pa < 0x1000_2000
}
