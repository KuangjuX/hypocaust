use crate::{mm::{MemorySet, VirtAddr, KERNEL_SPACE, MapPermission, PhysPageNum, VirtPageNum, PhysAddr, PageTable, PTEFlags},  trap::{TrapContext, trap_handler}, constants::layout::{TRAP_CONTEXT, kernel_stack_position, GUEST_KERNEL_VIRT_START_1, GUEST_KERNEL_VIRT_END_1, PAGE_SIZE}};
use crate::constants::csr;


mod switch;
mod context;
mod shadow_pgt;

use context::TaskContext;
use alloc::vec::Vec;
use switch::__switch;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;

use self::context::ShadowState;


lazy_static! {
    pub static ref GUEST_KERNEL_MANAGER: GuestKernelManager = GuestKernelManager::new();
}

pub struct GuestKernelManager {
    pub inner: UPSafeCell<GuestKernelManagerInner>
}

pub struct GuestKernelManagerInner {
    pub kernels: Vec<GuestKernel>,
    pub run_id: usize
}

impl GuestKernelManager {
    pub fn new() -> Self {
        Self {
           inner: unsafe{
            UPSafeCell::new(
                GuestKernelManagerInner{
                    kernels: Vec::new(),
                    run_id: 0
                }
            )
           }
        }
    }

    pub fn push(&self, kernel: GuestKernel) {
        self.inner.exclusive_access().kernels.push(kernel)
    }
}

pub fn run_guest_kernel() -> ! {
    let inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let id = inner.run_id;
    let guest_kernel = &inner.kernels[id];
    let task_cx_ptr = &guest_kernel.task_cx as *const TaskContext;
    drop(inner);
    let mut _unused = TaskContext::zero_init();
    // before this, we should drop local variables that must be dropped manually
    unsafe {
        __switch(&mut _unused as *mut _, task_cx_ptr);
    }
    panic!("unreachable in run_first_task!");
}

pub fn current_user_token() -> usize {
    let id = GUEST_KERNEL_MANAGER.inner.exclusive_access().run_id;
    GUEST_KERNEL_MANAGER.inner.exclusive_access().kernels[id].get_user_token()
}

pub fn current_trap_context_ppn() -> PhysPageNum {
    let id = GUEST_KERNEL_MANAGER.inner.exclusive_access().run_id;
    let kernel_memory = &GUEST_KERNEL_MANAGER.inner.exclusive_access().kernels[id].memory;
    let trap_context: VirtAddr = TRAP_CONTEXT.into();
    let trap_context_ppn= kernel_memory.translate(trap_context.floor()).unwrap().ppn();
    trap_context_ppn
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    let trap_context_ppn = current_trap_context_ppn();
    trap_context_ppn.get_mut() 
}

/// GVA -> GPA
pub fn translate_guest_vaddr(vaddr: usize) -> usize {
    let inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let kernel = &inner.kernels[inner.run_id];
    let state = &kernel.shadow_state;
    state.translate_guest_vaddr(vaddr)
}

/// GPA -> HVA
pub fn translate_guest_paddr(paddr: usize) -> usize {
    let inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let kernel = &inner.kernels[inner.run_id];
    let offset = paddr & 0xfff;
    let vpn: VirtPageNum = VirtAddr(paddr).floor();
    let ppn = kernel.memory.translate(vpn).unwrap().ppn();
    let vaddr: PhysAddr = ppn.into();
    let vaddr: usize = vaddr.into();
    vaddr + offset
}

pub fn get_shadow_csr(csr: usize) -> usize {
    let mut inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let id = inner.run_id;
    let shadow_state = &mut inner.kernels[id].shadow_state;
    match csr {
        csr::sstatus => { shadow_state.get_sstatus() }
        csr::stvec => { shadow_state.get_stvec() }
        csr::sie => { shadow_state.get_sie() }
        csr::sscratch => { shadow_state.get_sscratch() }
        csr::sepc => { shadow_state.get_sepc() }
        csr::scause => { shadow_state.get_scause() }
        csr::stval => { shadow_state.get_stval() }
        csr::satp => { shadow_state.get_satp() }
        _ => { panic!("[hypervisor] Unrecognized") }
    }
}

pub fn write_shadow_csr(csr: usize, val: usize) {
    let mut inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let id = inner.run_id;
    let shadow_state = &mut inner.kernels[id].shadow_state;
    match csr {
        csr::sstatus => { shadow_state.write_sstatus(val) }
        csr::stvec => { shadow_state.write_stvec(val) }
        csr::sie => { shadow_state.write_sie(val) }
        csr::sscratch => { shadow_state.write_sscratch(val) }
        csr::sepc => { shadow_state.write_sepc(val) }
        csr::scause => { shadow_state.write_scause(val) }
        csr::stval => { shadow_state.write_stval(val) }
        csr::satp => { shadow_state.write_satp(val) }
        _ => { panic!("[hypervisor] Unrecognized") }
    }
}

/// STEP:
/// 1. VMM intercepts guest OS setting the virtual satp
/// 2. VMM iterates over the guest page table, constructs a corresponding shadow page table
/// 3. In shadow PT, every guest physical address is translated into host virtual address(machine address)
/// 4. Finally, VMM sets the real satp to point to the shadow page table
pub fn satp_handler(satp: usize) {
    hdebug!("satp: {:#x}", satp);
    let root_gpa = (satp << 12) & 0x7f_ffff_ffff;
    let root_hva = translate_guest_paddr(root_gpa);
    let root_hppn =  KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(root_hva >> 12)).unwrap().ppn();
    hdebug!("root_gpa: {:#x}, root_hva: {:#x}, root_hppn: {:?}", root_gpa, root_hva, root_hppn);
    hdebug!("Build shadow page table......");
    let guest_page_table = PageTable::from_ppn(root_hppn);
    // 翻译的时候不能直接翻译，而要加一个偏移量，即将 guest virtual address -> host virtual address
    // 最终翻译的结果为 GPA (Guest Physical Address)
    let ppn = guest_page_table.translate_guest(0x80329.into()).unwrap().ppn();
    hdebug!("ppn: {:?}", ppn);
    // 构建影子页表
    let mut shadow_pgt = PageTable::new();
    // 将根目录页表中的所有映射转成影子页表
    for gva in (GUEST_KERNEL_VIRT_START_1..GUEST_KERNEL_VIRT_END_1).step_by(PAGE_SIZE) {
        let gvpn = VirtPageNum::from(gva >> 12);
        // let gppn = guest.memory.translate(gvpn);
        let gppn = guest_page_table.translate(gvpn);
        // 如果 guest ppn 存在且有效
        if let Some(gppn) = gppn {
            if gppn.is_valid() {
                let gpa = gppn.ppn().0 << 12;
                let hva = translate_guest_paddr(gpa);
                let hvpn = PhysPageNum::from(hva >> 12);
                let mut pte_flags = PTEFlags::V | PTEFlags::U;
                if gppn.readable() {
                    pte_flags |= PTEFlags::R;
                }
                if gppn.writable() {
                    pte_flags |= PTEFlags::W;
                }
                if gppn.executable() {
                    pte_flags |= PTEFlags::X;
                }
                shadow_pgt.map(gvpn, hvpn, pte_flags)
            }
        }
    }
    // assert_eq!(shadow_pgt.translate(0x80000.into()).unwrap().readable(), true);
    // assert_eq!(shadow_pgt.translate(0x80000.into()).unwrap().is_valid(), true);
    // assert_eq!(shadow_pgt.translate(0x80000.into()).unwrap().writable(), false);
    // assert_eq!(shadow_pgt.translate(0x80329.into()).unwrap().readable(), true);
    // assert_eq!(shadow_pgt.translate(0x80329.into()).unwrap().is_valid(), true);
    // assert_eq!(shadow_pgt.translate(0x80329.into()).unwrap().writable(), false);
    let mut inner = GUEST_KERNEL_MANAGER.inner.exclusive_access();
    let id = inner.run_id;
    let guest = &mut inner.kernels[id];
    guest.shadow_state.shadow_pgt = Some(shadow_pgt);
    hdebug!("Success to construct shadow page table......");
}

/// Guest Kernel 结构体
pub struct GuestKernel {
    /// guest kernel 内存映射，从 GPA -> HVA 转换
    pub memory: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub task_cx: TaskContext,
    pub shadow_state: ShadowState
}

impl GuestKernel {
    pub fn new(memory: MemorySet, index: usize) -> Self {
        // 获取中断上下文的物理地址
        let trap_cx_ppn = memory
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        // 获取内核栈地址
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(index);
        // 将内核栈地址进行映射
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let guest_kernel = Self { 
            memory,
            trap_cx_ppn,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            shadow_state: ShadowState::new()
        };
        // 获取中断上下文的地址
        let trap_cx : &mut TrapContext = guest_kernel.trap_cx_ppn.get_mut();
        *trap_cx = TrapContext::app_init_context(
            GUEST_KERNEL_VIRT_START_1,
            0,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        guest_kernel
    }

    pub fn get_user_token(&self) -> usize {
        if let Some(shadow_pgt) = &self.shadow_state.shadow_pgt {
            return shadow_pgt.token()
        }
        self.memory.token()
    }

}

