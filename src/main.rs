//! The main module and entrypoint
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)] 
// PR #16 (fix-bug/modern-rust-toolchain): legacy linker-symbol casts and global
// allocator storage are intentional low-level patterns. Keep them buildable
// while retaining deny(warnings) for warnings that are actionable here.
#![allow(function_casts_as_integer, static_mut_refs, dropping_references)]
#![deny(warnings)]


extern crate alloc;

#[macro_use]
extern crate bitflags;

#[path = "boards/qemu.rs"]
mod board;

#[macro_use]
mod console;
mod constants;
mod identity;
mod lang_items;
mod page_table;
mod sbi;
mod sync;
mod timer;
mod guest;
mod debug;
mod mm;
mod device_emu;
mod hypervisor;



use crate::constants::layout::{MAX_HOST_HARTS, PAGE_SIZE};
use crate::device_emu::DeviceBusConfig;
use crate::guest::{
    install_guest_fdt, GuestPayload, VcpuBootConfig, VirtualMachine, VmConfig,
};
use crate::hypervisor::{HYPOCAUST, HYPERVISOR_MEMORY};
use crate::identity::{GuestHartId, HartId, VcpuId, VmId};
use crate::mm::{LoadedGuestKernel, MemorySet};

// use fdt::Fdt;

#[link_section = ".initrd"]
#[cfg(feature = "embed_guest_kernel")]
static GUEST_KERNEL: [u8;include_bytes!("../guest_kernel").len()] = 
 *include_bytes!("../guest_kernel");

 #[cfg(not(feature = "embed_guest_kernel"))]
 static GUEST_KERNEL: [u8; 0] = [];

const HART_BOOT_STACK_SIZE: usize = 16 * PAGE_SIZE;

#[link_section = ".bss.stack"]
/// PR #37 (`fix-bug/per-hart-boot-stacks`) reserves one non-overlapping early
/// boot stack per supported Host hart.
static BOOT_STACK: [u8; HART_BOOT_STACK_SIZE * MAX_HOST_HARTS] =
    [0u8; HART_BOOT_STACK_SIZE * MAX_HOST_HARTS];

#[link_section = ".text.entry"]
#[export_name = "_start"]
// PR #16 (fix-bug/modern-rust-toolchain): naked_asm keeps the boot entry free of a
// compiler-generated prologue while using the current Rust naked-function API.
#[unsafe(naked)]
/// hypocaust entrypoint
pub unsafe extern "C" fn start() -> ! {
    core::arch::naked_asm!(
        // Reject an out-of-range hart before calculating a stack pointer.
        "li t4, {max_host_harts}",
        "bgeu a0, t4, 2f",
        // PR #38 keeps the Host hart ID in tp while Hypocaust code runs.
        "mv tp, a0",
        // prepare stack
        "la sp, {boot_stack}",
        "li t2, {hart_boot_stack_size}",
        "addi t3, a0, 1",
        "mul t2, t2, t3",
        "add sp, sp, t2",
        // enter hentry
        "call hentry",
        // Unsupported harts cannot safely enter Rust without a stack.
        "2:",
        "wfi",
        "j 2b",
        boot_stack = sym BOOT_STACK,
        hart_boot_stack_size = const HART_BOOT_STACK_SIZE,
        max_host_harts = const MAX_HOST_HARTS,
    )
}

/// clear BSS segment
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

#[no_mangle]
pub fn hentry(raw_hart_id: usize, device_tree_blob: usize) -> ! {
    let hart_id = HartId::new(raw_hart_id);
    if hart_id.is_boot() {
        clear_bss();
        hdebug!("Hello Hypocaust");
        hdebug!("hart_id: {}, device tree blob: {:#x}", hart_id.index(), device_tree_blob);
        let meta = hypervisor::fdt::MachineMeta::parse(device_tree_blob);
        let hart_count = meta.hart_count.min(MAX_HOST_HARTS);
        // 初始化堆及帧分配器
        hypervisor::hyp_alloc::heap_init();
        hypervisor::initialize_vmm(meta);
        {
            let mut hypervisor = HYPOCAUST.lock();
            let hypervisor = {&mut *hypervisor}.as_mut().unwrap();
            // PR #45 (`feature/multi-guest-qemu`) requires two distinct active
            // Host VirtIO devices before granting one backend to each VM.
            let virtio_backends = [
                hypervisor
                    .meta
                    .virtio
                    .get(0)
                    .expect("multi-Guest mode requires QEMU VirtIO backend 0")
                    .base_address,
                hypervisor
                    .meta
                    .virtio
                    .get(1)
                    .expect("multi-Guest mode requires QEMU VirtIO backend 1")
                    .base_address,
            ];
            hypervisor::trap::init();
            for (vm_index, host_virtio_base) in virtio_backends.into_iter().enumerate() {
                let vm_id = VmId::new(vm_index);
                let vcpu_id = VcpuId::new(vm_index);
                let guest_hart_id = GuestHartId::new(0);
                let config = VmConfig::new(
                    vm_id,
                    DeviceBusConfig::qemu_virtio_block(host_virtio_base),
                );
                let mut vm = VirtualMachine::new(config);
                // PR #45 loads the same example kernel into disjoint VM-owned
                // RAM slots; no Guest page can alias another VM's memory.
                // PR #51 (`feature/linux-image-loader`) detects the embedded
                // payload once per VM and carries its real entry point through
                // vCPU construction instead of assuming the xv6-rust address.
                let payload = GuestPayload::detect(&GUEST_KERNEL)
                    .expect("embedded Guest payload is neither ELF nor Linux Image");
                let LoadedGuestKernel {
                    memory_set: guest_kernel_memory,
                    entry_gpa,
                } = MemorySet::load_guest_kernel(payload, vm.guest_memory());
                mm::vm_init(&guest_kernel_memory);
                mm::guest_kernel_test(vm.guest_memory());
                let guest_fdt = install_guest_fdt(vm.guest_memory(), guest_hart_id);
                let user_guest_kernel_memory =
                    MemorySet::create_user_guest_kernel(&guest_kernel_memory);
                vm.add_vcpu(
                    user_guest_kernel_memory,
                    vcpu_id,
                    VcpuBootConfig::new(guest_hart_id, guest_fdt, entry_gpa),
                );
                hdebug!(
                    "configured VM {} with VirtIO backend {:#x} and DTB {:#x}",
                    vm_id.index(),
                    host_virtio_base,
                    guest_fdt,
                );
                hypervisor.add_vm(vm);
            }
            // Test the shared Host mapping after both Guest RAM slots have
            // been installed by PR #45.
            mm::remap_test();
        }
        // PR #38 (`feature/multivcpu-scheduler`) starts secondary Host harts
        // only after VM construction and Host mappings are globally visible.
        for index in 1..hart_count {
            sbi::start_hart(HartId::new(index), start as usize, device_tree_blob);
        }
        hypervisor::run_scheduler(hart_id)
    } else {
        // PR #38 installs the shared Host page table on each SBI-started hart
        // before the hart enters its independent scheduler loop.
        HYPERVISOR_MEMORY.exclusive_access().activate();
        hypervisor::trap::init();
        hdebug!("scheduler online on hart {}", hart_id.index());
        hypervisor::run_scheduler(hart_id)
    }
}
