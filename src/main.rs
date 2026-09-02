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
use crate::guest::{VirtualMachine, VmConfig};
use crate::hypervisor::{HYPOCAUST, HYPERVISOR_MEMORY};
use crate::identity::{GuestHartId, HartId, VcpuId};
use crate::mm::MemorySet;

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
            let vcpu_id = VcpuId::new(0);
            // PR #43 (`feature/vm-runtime-config`) makes the QEMU device
            // assignment visible at the VM construction boundary.
            let mut vm = VirtualMachine::new(VmConfig::qemu_default());
            // PR #36 (`feature/vm-guest-memory`) creates the VM-owned RAM slot
            // before loading the Guest so every mapping uses the same capability.
            let guest_kernel_memory =
                MemorySet::new_guest_kernel(&GUEST_KERNEL, vm.guest_memory());
            // 初始化虚拟内存
            mm::vm_init(&guest_kernel_memory);
            hypervisor::trap::init();
            // 测试重映射
            mm::remap_test();
            // 测试 guest kernel 内存映射
            mm::guest_kernel_test(vm.guest_memory());
            // 创建用户态的 guest kernel 内存空间
            let user_guest_kernel_memory =
                MemorySet::create_user_guest_kernel(&guest_kernel_memory);
            vm.add_vcpu(
                user_guest_kernel_memory,
                vcpu_id,
                GuestHartId::new(0),
            );
            hypervisor.add_vm(vm);
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
