use alloc::collections::{BTreeMap, VecDeque};

use crate::constants::layout::MAX_HOST_HARTS;
use crate::identity::{HartId, VcpuId, VcpuKey, VmId};

/// PR #38 (`feature/multivcpu-scheduler`) keeps vCPU lifecycle state in one
/// scheduler so a vCPU cannot be running on two Host harts simultaneously.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VcpuRunState {
    Ready,
    Running(HartId),
    Blocked,
    /// PR #55 (`fix-bug/guest-shutdown-isolation`) makes shutdown terminal:
    /// device interrupts must never wake a vCPU after its VM has stopped.
    Stopped,
}

pub struct Scheduler {
    run_queue: VecDeque<VcpuKey>,
    states: BTreeMap<VcpuId, (VcpuKey, VcpuRunState)>,
    current: [Option<VcpuKey>; MAX_HOST_HARTS],
    online: [bool; MAX_HOST_HARTS],
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            run_queue: VecDeque::new(),
            states: BTreeMap::new(),
            current: [None; MAX_HOST_HARTS],
            online: [false; MAX_HOST_HARTS],
        }
    }

    /// Record that a Host hart has installed the shared Host mappings and can
    /// safely receive scheduler IPIs introduced by PR #38.
    pub fn mark_hart_online(&mut self, hart_id: HartId) {
        *self
            .online
            .get_mut(hart_id.index())
            .expect("Host hart exceeds scheduler capacity") = true;
    }

    pub fn register(&mut self, key: VcpuKey) {
        assert!(!self.states.contains_key(&key.vcpu_id), "duplicate vCPU ID");
        self.states.insert(key.vcpu_id, (key, VcpuRunState::Ready));
        self.run_queue.push_back(key);
    }

    pub fn current(&self, hart_id: HartId) -> Option<VcpuKey> {
        self.current.get(hart_id.index()).copied().flatten()
    }

    pub fn schedule(&mut self, hart_id: HartId) -> Option<VcpuKey> {
        assert!(
            self.online.get(hart_id.index()).copied().unwrap_or(false),
            "cannot schedule work on an offline Host hart",
        );
        let slot = self
            .current
            .get_mut(hart_id.index())
            .expect("Host hart exceeds scheduler capacity");
        assert!(slot.is_none(), "Host hart already owns a running vCPU");

        while let Some(key) = self.run_queue.pop_front() {
            let (_, state) = self.states.get_mut(&key.vcpu_id).unwrap();
            if *state == VcpuRunState::Ready {
                *state = VcpuRunState::Running(hart_id);
                *slot = Some(key);
                return Some(key);
            }
        }
        None
    }

    pub fn preempt(&mut self, hart_id: HartId) -> Option<VcpuKey> {
        if let Some(key) = self.take_current(hart_id) {
            let (_, state) = self.states.get_mut(&key.vcpu_id).unwrap();
            *state = VcpuRunState::Ready;
            self.run_queue.push_back(key);
        }
        self.schedule(hart_id)
    }

    pub fn block_current(&mut self, hart_id: HartId) -> Option<VcpuKey> {
        if let Some(key) = self.take_current(hart_id) {
            let (_, state) = self.states.get_mut(&key.vcpu_id).unwrap();
            *state = VcpuRunState::Blocked;
        }
        self.schedule(hart_id)
    }

    /// Stop every vCPU belonging to the current VM, then select unrelated work.
    ///
    /// PR #55 removes stopped entries from the ready queue and gives them a
    /// terminal state. The current implementation has one vCPU per VM; the
    /// assertion prevents a future SMP VM from being partially stopped until
    /// cross-hart stop coordination is implemented explicitly.
    pub fn stop_current_vm(&mut self, hart_id: HartId) -> (VmId, Option<VcpuKey>) {
        let current = self
            .take_current(hart_id)
            .expect("Host hart has no VM to stop");
        let vm_id = current.vm_id;
        for (key, state) in self.states.values_mut() {
            if key.vm_id != vm_id {
                continue;
            }
            assert!(
                !matches!(*state, VcpuRunState::Running(owner) if owner != hart_id),
                "stopping a VM with a vCPU running on another Host hart is unsupported",
            );
            *state = VcpuRunState::Stopped;
        }
        self.run_queue.retain(|key| key.vm_id != vm_id);
        (vm_id, self.schedule(hart_id))
    }

    /// Make a blocked vCPU runnable and return an idle hart that should receive
    /// an SBI IPI. A running or already-ready vCPU is left unchanged.
    pub fn wake(&mut self, key: VcpuKey) -> Option<HartId> {
        let (registered_key, state) = self
            .states
            .get_mut(&key.vcpu_id)
            .expect("waking an unknown vCPU");
        assert_eq!(*registered_key, key, "vCPU key does not match registration");
        if *state != VcpuRunState::Blocked {
            return None;
        }
        *state = VcpuRunState::Ready;
        self.run_queue.push_back(key);
        self.current
            .iter()
            .zip(self.online.iter())
            .position(|(current, online)| current.is_none() && *online)
            .map(HartId::new)
    }

    /// PR #40 chooses the Host hart that must leave Guest or idle execution to
    /// observe a newly injected interrupt. Blocked vCPUs become ready first.
    pub fn interrupt_target(&mut self, key: VcpuKey) -> Option<HartId> {
        let (registered_key, state) = self
            .states
            .get(&key.vcpu_id)
            .expect("interrupting an unknown vCPU");
        assert_eq!(*registered_key, key, "vCPU key does not match registration");
        match *state {
            VcpuRunState::Running(hart_id) => Some(hart_id),
            VcpuRunState::Blocked => self.wake(key),
            VcpuRunState::Ready | VcpuRunState::Stopped => None,
        }
    }

    fn take_current(&mut self, hart_id: HartId) -> Option<VcpuKey> {
        self.current
            .get_mut(hart_id.index())
            .expect("Host hart exceeds scheduler capacity")
            .take()
    }
}

/// Exercise the scheduler state machine before Hypocaust publishes its global
/// instance. PR #38 keeps this freestanding because the bare-metal target has
/// no standard Rust test harness.
pub fn self_test() {
    let mut scheduler = Scheduler::new();
    let first = VcpuKey::new(crate::identity::VmId::new(0), VcpuId::new(0));
    let second = VcpuKey::new(crate::identity::VmId::new(1), VcpuId::new(1));

    scheduler.mark_hart_online(HartId::new(0));
    scheduler.mark_hart_online(HartId::new(1));
    scheduler.register(first);
    scheduler.register(second);
    assert_eq!(scheduler.schedule(HartId::new(0)), Some(first));
    assert_eq!(scheduler.schedule(HartId::new(1)), Some(second));
    assert_eq!(scheduler.interrupt_target(first), Some(HartId::new(0)));
    assert_eq!(scheduler.block_current(HartId::new(1)), None);
    assert_eq!(scheduler.interrupt_target(second), Some(HartId::new(1)));
    assert_eq!(scheduler.schedule(HartId::new(1)), Some(second));
    assert_eq!(scheduler.preempt(HartId::new(0)), Some(first));

    // PR #55 verifies that stopping VM 0 selects VM 1 and that a later device
    // interrupt cannot resurrect the terminal VM 0 vCPU.
    let mut shutdown_scheduler = Scheduler::new();
    shutdown_scheduler.mark_hart_online(HartId::new(0));
    shutdown_scheduler.register(first);
    shutdown_scheduler.register(second);
    assert_eq!(shutdown_scheduler.schedule(HartId::new(0)), Some(first));
    assert_eq!(
        shutdown_scheduler.stop_current_vm(HartId::new(0)),
        (first.vm_id, Some(second)),
    );
    assert_eq!(shutdown_scheduler.interrupt_target(first), None);
}
