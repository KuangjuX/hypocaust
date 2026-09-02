//! VM-owned Platform-Level Interrupt Controller emulation.
//!
//! PR #41 (`feature/per-vm-virtual-plic`) implements the PLIC register subset
//! required for priority, enable, threshold, claim, and completion handling.

use crate::constants::layout::MAX_HOST_HARTS;

pub const PLIC_GPA: usize = 0x0c00_0000;
pub const PLIC_SIZE: usize = 0x0400_0000;
pub const VIRTIO_BLOCK_IRQ: u32 = 1;

const MAX_SOURCES: usize = 32;
const PRIORITY_END: usize = 0x1000;
const PENDING_OFFSET: usize = 0x1000;
const ENABLE_BASE: usize = 0x2080;
const ENABLE_STRIDE: usize = 0x100;
const CONTEXT_BASE: usize = 0x20_1000;
const CONTEXT_STRIDE: usize = 0x2000;
const CLAIM_OFFSET: usize = 4;

pub struct VirtualPlic {
    priorities: [u32; MAX_SOURCES],
    pending: u32,
    enables: [u32; MAX_HOST_HARTS],
    thresholds: [u32; MAX_HOST_HARTS],
    claimed: [u32; MAX_HOST_HARTS],
}

impl VirtualPlic {
    pub const fn new() -> Self {
        Self {
            priorities: [0; MAX_SOURCES],
            pending: 0,
            enables: [0; MAX_HOST_HARTS],
            thresholds: [0; MAX_HOST_HARTS],
            claimed: [0; MAX_HOST_HARTS],
        }
    }

    pub fn contains(&self, guest_address: usize) -> bool {
        guest_address >= PLIC_GPA && guest_address - PLIC_GPA < PLIC_SIZE
    }

    /// PR #41 raises one VM-local source. The caller can use the return value
    /// to inject SEIP only when that Guest context can currently claim it.
    pub fn raise(&mut self, source: u32, context: usize) -> bool {
        assert!(context < MAX_HOST_HARTS, "virtual PLIC context out of range");
        let bit = Self::source_bit(source).expect("virtual PLIC source out of range");
        self.pending |= bit;
        self.next_claim(context).is_some()
    }

    /// PR #41 deasserts a level-triggered source without changing any other
    /// VM's controller or any unrelated source in this controller.
    pub fn lower(&mut self, source: u32) {
        let bit = Self::source_bit(source).expect("virtual PLIC source out of range");
        self.pending &= !bit;
    }

    /// PR #41 reports the level of one Guest context's external-interrupt
    /// output after priority, enable, and threshold filtering.
    pub fn has_interrupt(&self, context: usize) -> bool {
        self.next_claim(context).is_some()
    }

    pub fn read_u32(&mut self, guest_address: usize) -> Option<u32> {
        let offset = self.offset(guest_address)?;
        if offset < PRIORITY_END && offset % 4 == 0 {
            return self.priorities.get(offset / 4).copied();
        }
        if offset == PENDING_OFFSET {
            return Some(self.pending);
        }
        if let Some(context) = Self::enable_context(offset) {
            return self.enables.get(context).copied();
        }
        if let Some((context, register)) = Self::context_register(offset) {
            return match register {
                0 => self.thresholds.get(context).copied(),
                CLAIM_OFFSET => Some(self.claim(context)),
                _ => None,
            };
        }
        None
    }

    pub fn write_u32(&mut self, guest_address: usize, value: u32) -> bool {
        let Some(offset) = self.offset(guest_address) else {
            return false;
        };
        if offset < PRIORITY_END && offset % 4 == 0 {
            let source = offset / 4;
            if source == 0 || source >= MAX_SOURCES {
                return source == 0;
            }
            self.priorities[source] = value;
            return true;
        }
        if let Some(context) = Self::enable_context(offset) {
            let Some(enables) = self.enables.get_mut(context) else {
                return false;
            };
            *enables = value & !1;
            return true;
        }
        if let Some((context, register)) = Self::context_register(offset) {
            return match register {
                0 => match self.thresholds.get_mut(context) {
                    Some(threshold) => {
                        *threshold = value;
                        true
                    }
                    None => false,
                },
                CLAIM_OFFSET => self.complete(context, value),
                _ => false,
            };
        }
        false
    }

    fn claim(&mut self, context: usize) -> u32 {
        let Some(source) = self.next_claim(context) else {
            return 0;
        };
        let bit = 1u32 << source;
        self.pending &= !bit;
        self.claimed[context] |= bit;
        source as u32
    }

    fn complete(&mut self, context: usize, source: u32) -> bool {
        let Some(bit) = Self::source_bit(source) else {
            return false;
        };
        let Some(claimed) = self.claimed.get_mut(context) else {
            return false;
        };
        if *claimed & bit == 0 {
            return false;
        }
        *claimed &= !bit;
        true
    }

    /// Select the highest-priority enabled source, breaking ties in favor of
    /// the lower source ID as required by the PLIC architecture.
    fn next_claim(&self, context: usize) -> Option<usize> {
        let enabled = *self.enables.get(context)? & self.pending;
        let threshold = *self.thresholds.get(context)?;
        (1..MAX_SOURCES)
            .filter(|source| enabled & (1u32 << source) != 0)
            .filter(|source| self.priorities[*source] > threshold)
            .max_by(|left, right| {
                self.priorities[*left]
                    .cmp(&self.priorities[*right])
                    .then_with(|| right.cmp(left))
            })
    }

    fn source_bit(source: u32) -> Option<u32> {
        if source == 0 || source as usize >= MAX_SOURCES {
            None
        } else {
            Some(1u32 << source)
        }
    }

    fn offset(&self, guest_address: usize) -> Option<usize> {
        self.contains(guest_address)
            .then_some(guest_address - PLIC_GPA)
    }

    fn enable_context(offset: usize) -> Option<usize> {
        if offset < ENABLE_BASE {
            return None;
        }
        let relative = offset - ENABLE_BASE;
        (relative % ENABLE_STRIDE == 0).then_some(relative / ENABLE_STRIDE)
    }

    fn context_register(offset: usize) -> Option<(usize, usize)> {
        if offset < CONTEXT_BASE {
            return None;
        }
        let relative = offset - CONTEXT_BASE;
        Some((relative / CONTEXT_STRIDE, relative % CONTEXT_STRIDE))
    }
}

/// PR #41 validates register routing and claim/complete state transitions at
/// startup because Hypocaust's bare-metal target has no standard test runner.
pub fn self_test() {
    let mut plic = VirtualPlic::new();
    let priority = PLIC_GPA + VIRTIO_BLOCK_IRQ as usize * 4;
    let enable = PLIC_GPA + ENABLE_BASE;
    let threshold = PLIC_GPA + CONTEXT_BASE;
    let claim = threshold + CLAIM_OFFSET;

    assert!(plic.write_u32(priority, 1));
    assert!(plic.write_u32(enable, 1 << VIRTIO_BLOCK_IRQ));
    assert!(plic.write_u32(threshold, 0));
    assert!(plic.raise(VIRTIO_BLOCK_IRQ, 0));
    assert_eq!(plic.read_u32(PLIC_GPA + PENDING_OFFSET), Some(2));
    assert_eq!(plic.read_u32(claim), Some(VIRTIO_BLOCK_IRQ));
    assert_eq!(plic.read_u32(PLIC_GPA + PENDING_OFFSET), Some(0));
    assert!(plic.write_u32(claim, VIRTIO_BLOCK_IRQ));
}
