use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::cell::UnsafeCell;

use crate::debug::{PageDebug};
use crate::mm::KERNEL_SPACE;
use crate::page_table::{PageTable, VirtPageNum, PTEFlags, PageTableEntry, PhysPageNum};
use crate::constants::layout::{ PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, GUEST_KERNEL_VIRT_START, GUEST_KERNEL_VIRT_END, GUEST_TRAMPOLINE, GUEST_TRAP_CONTEXT, KERNEL_STACK_SIZE };
use super::GuestKernel;

/// 内存信息，用于帮助做地址映射
#[allow(unused)]
mod segment_layout {
    pub const HART_SEGMENT_SIZE: usize = 128 * 1024 * 1024;
}



/// 页表(影子页表类型)
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum PageTableRoot {
    /// Guest Physical Address
    GPA,
    /// Guest Virtual Address
    GVA,
    /// User Virtual Address
    UVA
}

// pub struct Rmap {
//     pub rmap: BTreeMap<PhysPageNum, usize>
// }

pub struct ShadowPageTableInfo<P: PageTable + PageDebug> {
    pub mode: PageTableRoot,
    /// 客户页表对应的 `satp`
    pub satp: usize,
    /// shadow page table
    pub spt: P,
    /// guest page table
    pub gpt: P,
    /// guest page table 虚拟页号，用于同步页表
    pub vpns: Vec<VirtPageNum>
    // /// 反向页表，用于从物理地址找回虚拟地址
    // pub rmap: Option<Rmap>
}

impl<P> ShadowPageTableInfo<P> where P: PageDebug + PageTable {
    pub fn new(satp: usize, spt: P, gpt: P,  vpns: Vec<VirtPageNum>, mode: PageTableRoot) -> Self {
        Self {
            mode,
            satp,
            spt,
            gpt,
            vpns
        }
    }

    pub fn token(&self) -> usize {
        self.spt.token()
    }
}

pub struct ShadowPageTables<P: PageTable + PageDebug> {
    pub page_tables: UnsafeCell<VecDeque<ShadowPageTableInfo<P>>>
}

impl<P> ShadowPageTables<P> where P: PageDebug + PageTable {
    pub const fn new() -> Self {
        Self {
            page_tables: UnsafeCell::new(VecDeque::new())
        }
    }

    pub fn inner(&self) -> &mut VecDeque<ShadowPageTableInfo<P>> {
        unsafe{ &mut *self.page_tables.get() }
    }

    pub fn push(&self, shadow_page_table: ShadowPageTableInfo<P>) {
        let inner = self.inner();
        inner.push_back(shadow_page_table);
    }

    pub fn find_shadow_page_table(&self, satp: usize) -> Option<&ShadowPageTableInfo<P>> {
        let inner = self.inner();
        for item in inner.iter() {
            if item.satp == satp {
                return Some(item)
            }
        }
        None
    }

    pub fn find_shadow_page_table_mut(&self, satp: usize) -> Option<&mut ShadowPageTableInfo<P>> {
        let inner = self.inner();
        for item in inner.iter_mut() {
            if item.satp == satp {
                return Some(item)
            }
        }
        None
    }

    pub fn guest_page_table(&self) -> Option<&ShadowPageTableInfo<P>> {
        let inner = self.inner();
        for item in inner.iter() {
            if item.mode == PageTableRoot::GVA {
                return Some(item)
            }
        }
        None
    }

    pub fn guest_page_table_mut(&self) -> Option<&mut ShadowPageTableInfo<P>> {
        let inner = self.inner();
        for item in inner.iter_mut() {
            if item.mode == PageTableRoot::GVA {
                return Some(item)
            }
        }
        None
    }

    pub fn spt_by_vpn(&self, vpn: VirtPageNum) -> Option<&ShadowPageTableInfo<P>> {
        let inner = self.inner();
        for spt in inner.iter() {
            // for v in spt.vpns {
            //     if v == vpn { return Some(spt) }
            // }

            if spt.vpns.iter().position(|&v| v == vpn).is_some(){ return Some(spt)}
        }
        None
    }

    pub fn spt_by_vpn_mut(&self, vpn: VirtPageNum) -> Option<&mut ShadowPageTableInfo<P>> {
        let inner = self.inner();
        for spt in inner.iter_mut() {
            // for v in spt.vpns {
            //     if v == vpn { return Some(spt) }
            // }
            if spt.vpns.iter().position(|&v| v == vpn).is_some(){ return Some(spt)}
        }
        None
    }

}

pub fn gpa2hpa(va: usize, hart_id: usize) -> usize {
    va + (hart_id + 1) * segment_layout::HART_SEGMENT_SIZE
}

pub fn hpa2gpa(pa: usize, hart_id: usize) -> usize {
    pa - (hart_id + 1) * segment_layout::HART_SEGMENT_SIZE
}

pub fn page_table_mode<P: PageTable>(page_table: P, hart_id: usize) -> PageTableRoot {
    if page_table.translate_guest(VirtPageNum::from(GUEST_KERNEL_VIRT_START >> 12), hart_id).is_some() {
        return PageTableRoot::GVA
    }
    PageTableRoot::UVA
}

fn map_guest_address<P: PageTable>(hart_id: usize, va: usize, gpt: &P, spt: &mut P, flags: Option<PTEFlags>) {
    let gvpn = VirtPageNum::from(va >> 12);
    if let Some(gpte) = gpt.translate_guest(gvpn, hart_id) {
        if gpte.is_valid() {
            let gppn = gpte.ppn();
            let hppn = PhysPageNum::from(gpa2hpa(gppn.0 << 12, hart_id) >> 12);
            if let Some(flags) = flags {
                spt.map(gvpn, hppn, flags);
            }else{
                let mut flags = PTEFlags::U;
                if gpte.readable() {
                    flags |= PTEFlags::R;
                }
                if gpte.writable() {
                    flags |= PTEFlags::W;
                }
                if gpte.executable() {
                    flags |= PTEFlags::X;
                }
                spt.map(gvpn, hppn, flags);
            }
        }
    }
}

/// 对于页表的 PTE 的标志位应当标志为只读，用来同步 Guest Page Table 和 Shadow Page Table
pub fn map_page_table<P: PageTable>(hart_id: usize, root_gpa: usize, spt: &mut P, vpns: &mut Vec<VirtPageNum>) {
    let root_gvpn = VirtPageNum::from(root_gpa >> 12);
    hdebug!("root vpn: {:#x}", root_gvpn.0);
    // 广度优先搜索遍历所有页表
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    queue.push_back(root_gvpn); 
    for index in 0..=2 {
        while !queue.is_empty() {
            let vpn = queue.pop_front().unwrap();
            // hdebug!("page table vpn: {:#x}", vpn.0);
            vpns.push(vpn);
            let ppn = PhysPageNum::from(gpa2hpa(vpn.0 << 12, hart_id) >> 12);
            // 如果 `spt` 中的该虚拟页已经被映射了，先解除再进行映射
            if let Some(pte) = spt.translate(vpn) {
                if pte.is_valid(){ spt.unmap(vpn); }
            }
            spt.map(vpn, ppn, PTEFlags::R | PTEFlags::U);
            let ptes = ppn.get_pte_array();
            for pte in ptes {
                if pte.is_valid() {
                    buffer.push(VirtPageNum::from(pte.ppn().0))
                }
            } 
        }
        if index < 2 {
            while !buffer.is_empty() {
                queue.push_back(buffer.pop().unwrap());
            }
        }
    }
}


pub fn update_page_table_readonly<P: PageTable>(hart_id: usize, guest_spt: &mut P, gpt: &P) {
    let mut queue = VecDeque::new();
    let mut buffer = Vec::new();
    let root_vpn = VirtPageNum::from(hpa2gpa(gpt.root_ppn().0 << 12, hart_id) >> 12);
    queue.push_back(root_vpn); 
    for index in 0..=2 {
        while !queue.is_empty() {
            let vpn = queue.pop_front().unwrap();
            let ppn = PhysPageNum::from(gpa2hpa(vpn.0 << 12, hart_id) >> 12);
            // 如果 `spt` 中的该虚拟页已经被映射了，先解除再进行映射
            if let Some(gpte) = guest_spt.translate(vpn) {
                // hdebug!("Before judge");
                if gpte.is_valid() && gpte.writable() { 
                    guest_spt.unmap(vpn); 
                    guest_spt.map(vpn, ppn, PTEFlags::R | PTEFlags::U);
                }
                // hdebug!("After judge");
            }else{
                guest_spt.map(vpn, ppn, PTEFlags::R | PTEFlags::U);
            }
            if ppn.0 >= 0x8000_0000 && ppn.0 <= 0x8800_0000 {
                let ptes = ppn.get_pte_array();
                // hdebug!("ptes addr: {:#x}", ptes.as_ptr() as usize);
                for pte in ptes {
                    if pte.is_valid() {
                        buffer.push(VirtPageNum::from(pte.ppn().0))
                    }
                } 
            }
            
        }
        if index < 2 {
            while !buffer.is_empty() {
                queue.push_back(buffer.pop().unwrap());
            }
        }
    }
}


pub fn update_page_table<P: PageTable>(hart_id: usize, addr: usize, spt: &mut P, gpt: &P, vpns: &mut Vec<VirtPageNum>) {
    // User Spcae/Trap Context/Trampoline
    // 检查 spt 与 gpt 是否同步
    // 查看是否可以翻译 spt
    let vpn = VirtPageNum::from(addr >> 12);
    if let Some(gpte) = gpt.translate_guest(vpn, hart_id) {
        if gpte.is_valid() {
            // gva --------> gpa --------> hpa
            let hpa = gpa2hpa(gpte.ppn().0 << 12, hart_id);
            let hppn = PhysPageNum::from(hpa >> 12);
            let mut flags = PTEFlags::U;
            if gpte.readable(){ flags |= PTEFlags::R; }
            if gpte.writable(){ flags |= PTEFlags::W; }
            if gpte.executable(){ flags |= PTEFlags::X }
            match spt.translate(vpn) {
                Some(spte) => {
                    if !spte.is_valid() {
                        // 如果该页面映射不是有效的
                        // gpa -> hpa -> hppn
                        spt.map(vpn, hppn, flags);
                        if vpns.iter().position(|&item| item == vpn).is_none() {
                            vpns.push(vpn);
                        }
                    }else{
                        // 判断二者映射是否相同
                        if hppn != spte.ppn() {
                            // 二者不同，需要重新映射
                            spt.map(vpn, hppn, flags);
                            if vpns.iter().position(|&item| item == vpn).is_none() {
                                vpns.push(vpn);
                            }
                        } 
                    }
                },
                None => {
                    // 如果映射失败，需要重新映射
                    spt.map(vpn, hppn, flags);
                    if vpns.iter().position(|&item| item == vpn).is_none() {
                        vpns.push(vpn);
                    }
                }
            }
        }
    }
}


/// 同步 kernel guest page table 与 shadow page table
/// STEP:
/// 1. 搜索内核地址空间，查看是否存在 gpt 与 spt 不一致的地方，并映射
/// 2. 若当前的 `vpn` 在 `vpns` 中不存在，加入 `vpns` 中
/// 3. 将当前的 `vpn` 在 `guest_spt` 中设置为只读(`spt` 就为 `guest_spt`)
/// TODO: fix hardcoded address space to memory region
pub fn synchronize_kernel_page_table<P: PageTable>(hart_id: usize, spt: &mut P, gpt: &P, vpns: &mut Vec<VirtPageNum>) {
    // Kernel Space
    // for addr in (GUEST_KERNEL_VIRT_START..GUEST_KERNEL_VIRT_END).step_by(PAGE_SIZE) {
    //     update_page_table(hart_id, addr, spt, gpt, vpns)
    // }
    // Trap Context
    let kernel_stack_top = GUEST_TRAP_CONTEXT;
    let kernel_stack_bottom = kernel_stack_top - 50 * (KERNEL_STACK_SIZE + PAGE_SIZE);
    for va in (kernel_stack_bottom..=kernel_stack_top).step_by(PAGE_SIZE) {
        update_page_table(hart_id, va, spt, gpt, vpns);
    }
    // TRAMPOLINE
    update_page_table(hart_id, GUEST_TRAMPOLINE, spt, gpt, vpns);
    // 将 gpt 中的页表在 kernel spt 中映射为只读
    update_page_table_readonly(hart_id, spt, gpt);
}


/// 同步 user guest page table 与 shadow page table
/// STEP:
/// 1. 搜索用户地址空间，查看是否存在 gpt 与 spt 不一致的地方，并映射
/// 2. 若当前的 `vpn` 在 `vpns` 中不存在，加入 `vpns` 中
/// 3. 将当前的 `vpn` 在 `guest_spt` 中设置为只读
/// TODO: fix hardcoded address space to memory region
pub fn synchronize_user_page_table<P: PageTable>(
    hart_id: usize, spt: &mut P, guest_spt: &mut P,
    gpt: &P, vpns: &mut Vec<VirtPageNum>
) {
    // User Space
    // hdebug!("Before sync user space");
    for addr in (0x10000..0x80000).step_by(PAGE_SIZE) {
        update_page_table(hart_id, addr, spt, gpt, vpns);
    }
    // hdebug!("After sync user space");
    // Trap Context
    update_page_table(hart_id, GUEST_TRAP_CONTEXT, spt, gpt, vpns);
    // hdebug!("After sync trap context");
    // TRAMPOLINE
    update_page_table(hart_id, GUEST_TRAMPOLINE, spt, gpt, vpns);
    // hdebug!("After sync trampoline");
    // 将 gpt 中的页表在 kernel spt 中映射为只读
    update_page_table_readonly(hart_id, guest_spt, gpt);
    // hdebug!("After sync page table");
}



impl<P> GuestKernel<P> where P: PageDebug + PageTable {
    /// GPA -> HPA
    pub fn translate_guest_paddr(&self, paddr: usize) -> Option<usize> {
        let offset = paddr & 0xfff;
        let vpn: VirtPageNum = VirtPageNum::from(paddr >> 12);
        let pte = self.translate_guest_ppte(vpn);
        if let Some(pte) = pte {
            return Some((pte.ppn(). 0 << 12) + offset)
        }
        None
    }

    /// GVA -> HPA
    pub fn translate_guest_vaddr(&self, vaddr: usize) -> Option<usize> {
        let offset = vaddr & 0xfff;
        let vpn = VirtPageNum::from(vaddr >> 12);
        let pte = self.translate_guest_vpte(vpn);
        if let Some(pte) = pte {
            return Some((pte.ppn(). 0 << 12) + offset)
        }
        None
    }

    pub fn translate_guest_ppte(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.memory.translate(vpn)
    }

    pub fn translate_guest_vpte(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        if let Some(info) = self.shadow_state.shadow_page_tables.find_shadow_page_table(self.shadow_state.get_satp()) {
            // 由于 GHA 与 GPA 是同等映射的，因此翻译成的物理地址可以直接当虚拟地址用
            let pte = info.spt.translate(vpn);
            pte
        }else{
            self.translate_guest_ppte(vpn)
        }
    }

    pub fn translate_valid_guest_vaddr(&self, vaddr: usize) -> Option<usize> {
        let offset = vaddr & 0xfff;
        let vpn = VirtPageNum::from(vaddr >> 12);
        let pte = self.translate_guest_vpte(vpn);
        if let Some(pte) = pte {
            if !pte.is_valid(){ return None }
            return Some((pte.ppn(). 0 << 12) + offset)
        }
        None
    }

    /// 验证需要映射的内存是否为客户页表的页表项，若为页表项，则将
    /// 权限位设置为不可写，以便在 Guest OS 修改页表项时陷入 VMM
    pub fn is_guest_page_table(&self, vaddr: usize) -> bool {
        // let spt = self.shadow_state.shadow_page_tables.guest_page_table().unwrap();
        let trap_vpn = VirtPageNum::from(vaddr >> 12);
        let spts = self.shadow_state.shadow_page_tables.inner();
        for page_info in spts.iter() {
            if page_info.vpns.iter().position(|&vpn| vpn == trap_vpn).is_some(){ return true }
        }
        false
    }

    /// 根据 satp 构建影子页表
    /// 需要将 GVA -> HPA
    pub fn make_shadow_page_table(&mut self, satp: usize) {
        if self.shadow_state.shadow_page_tables.find_shadow_page_table(satp).is_none() {
            // 如果影子页表中没有发现，新建影子页表
            // 根据 satp 获取 guest kernel 根页表的物理地址
            let hart_id = self.index;
            let root_gpa = (satp & 0xfff_ffff_ffff) << 12;
            let root_hppn = PhysPageNum::from(gpa2hpa(root_gpa, hart_id) >> 12);
            let gpt = P::from_ppn(root_hppn);
            let mut vpns = Vec::new();
            // 翻译的时候不能直接翻译，因为此时取出的 pte 都是 Guest OS 的物理地址，需要将 pte 翻译成 hypervisor 的地址
            // 即将 guest virtual address -> host virtual address
            // 最终翻译的结果为 GPA (Guest Physical Address)
            // 构建影子页表
            let mut spt = PageTable::new();
            let mode;
            // 根据页表是否可读内核地址空间判断是 `GVA` 还是 `UVA`
            match page_table_mode(gpt.clone(), hart_id) {
                PageTableRoot::GVA => {
                    // 尝试映射内核地址空间
                    for va in (GUEST_KERNEL_VIRT_START..GUEST_KERNEL_VIRT_END).step_by(PAGE_SIZE) {
                        map_guest_address(hart_id, va, &gpt, &mut spt, None);
                    }
                    // 映射客户页表
                    map_page_table(hart_id, root_gpa, &mut spt, &mut vpns);
                    // 尝试映射用户空间的跳板页
                    map_guest_address(hart_id, GUEST_TRAMPOLINE, &gpt, &mut spt, Some(PTEFlags::U | PTEFlags::R | PTEFlags::X));
                    // 尝试映射用户空间 Trap Context
                    map_guest_address(hart_id, GUEST_TRAP_CONTEXT, &gpt, &mut spt, Some(PTEFlags::U | PTEFlags::R | PTEFlags::W));
                    map_guest_address(hart_id, GUEST_TRAP_CONTEXT - PAGE_SIZE, &gpt, &mut spt, Some(PTEFlags::U | PTEFlags::R | PTEFlags::W));
                    // 将 mode 设置为 `GVA`
                    mode = PageTableRoot::GVA;
                    // hdebug!("{:#x}", GUEST_TRAP_CONTEXT - PAGE_SIZE);
                    // let vpn = VirtPageNum::from((GUEST_TRAP_CONTEXT - PAGE_SIZE) >> 12);
                    // assert!(gpt.translate(vpn).unwrap().writable() == true);
                    // assert!(spt.translate(VirtPageNum::from(vpn)).unwrap().writable() == true);
                }
                PageTableRoot::UVA => {
                    // 尝试映射用户地址空间
                    for va in (0x1_0000..0x8_0000).step_by(PAGE_SIZE) {
                        map_guest_address(hart_id, va, &gpt, &mut spt, None);
                    }
                    // 尝试映射用户空间的跳板页
                    map_guest_address(hart_id, GUEST_TRAMPOLINE, &gpt, &mut spt, Some(PTEFlags::U | PTEFlags::R | PTEFlags::X));
                    // 尝试映射用户空间 Trap Context
                    map_guest_address(hart_id, GUEST_TRAP_CONTEXT, &gpt, &mut spt, Some(PTEFlags::U | PTEFlags::R | PTEFlags::W));
                    // 将 mode 设置为 `UVA`
                    mode = PageTableRoot::UVA;
                    // 同步 guest spt,即将用户页表设置为只读
                    let guest_spt = self.shadow_state.shadow_page_tables.guest_page_table_mut().unwrap();                   
                    // 将用户页表设置为只读
                    map_page_table(hart_id, root_gpa, &mut guest_spt.spt, &mut vpns);
                    
                }
                _ => unreachable!()
            }

            // 无论是 guest spt 还是 user spt 都要映射跳板页与 Trap Context
            let trampoline_hppn = KERNEL_SPACE.exclusive_access().translate(VirtPageNum::from(TRAMPOLINE >> 12)).unwrap().ppn();
            spt.map(VirtPageNum::from(TRAMPOLINE >> 12), trampoline_hppn, PTEFlags::R | PTEFlags::X);

            let trapctx_hvpn = VirtPageNum::from(self.translate_guest_paddr(TRAP_CONTEXT).unwrap() >> 12);
            let trapctx_hppn = KERNEL_SPACE.exclusive_access().translate(trapctx_hvpn).unwrap().ppn();
            spt.map(VirtPageNum::from(TRAP_CONTEXT >> 12), trapctx_hppn, PTEFlags::R | PTEFlags::W);

            // 修改 `shadow page table`
            // hdebug!("Make new SPT(satp -> {:#x}, spt -> {:#x}) ", satp, shadow_pgt.token());
            let spt_info = ShadowPageTableInfo::new(satp, spt, gpt, vpns, mode);
            self.shadow_state.shadow_page_tables.push(spt_info);
        }
    }

    // /// 同步 `shadow page table` & `guest page table`
    // pub fn sync_shadow_page_table(&mut self, mut vaddr: usize, pte: PageTableEntry, ctx: &TrapContext) {
    //     let satp = self.shadow_state.get_satp();
    //     let root_ppn = PhysPageNum::from(satp & 0xfff_ffff_ffff);
    //     let hart_id = self.index;
    //     if let Some(spt) = self.shadow_state.shadow_page_tables.guest_page_table_mut() {
    //         if let Some(rmap) = &mut spt.rmap {
    //             let mut vpn: usize = 0;
    //             let mut ppn = PhysPageNum::from(vaddr >> 12);
    //             let mut i = 0;
    //             while ppn != root_ppn {
    //                 let index = (vaddr & 0xfff) / size_of::<PageTableEntry>();
    //                 vpn.set_bits((i * 9)..(i * 9) + 9, index);
    //                 if let Some(value) = rmap.rmap.get(&ppn) {
    //                     vaddr = *value;
    //                     ppn = PhysPageNum::from(vaddr >> 12);
    //                     i += 1;
    //                 }else{
    //                     break;
    //                 }
    //             }
    //             let index = (vaddr & 0xfff) / size_of::<PageTableEntry>();
    //             vpn.set_bits((i * 9)..(i * 9) + 9, index);
    //             if i == 2 {
    //                 // 生成虚拟页号
    //                 let vpn = VirtPageNum::from(vpn);
    //                 if pte.is_valid() {
    //                     let mut flags = PTEFlags::U;
    //                     if pte.readable(){ flags |= PTEFlags::R };
    //                     if pte.writable(){ flags |= PTEFlags::W };
    //                     if pte.executable(){ flags |= PTEFlags::X };
    //                     let pa = gpa2hpa(pte.ppn().0 << 12, hart_id);
    //                     let ppn = PhysPageNum::from(pa >> 12);
    //                     hdebug!("{:#x} -> {:#x}, pc: {:#x}, stval: {:#x}", vpn.0, ppn.0, ctx.sepc, stval::read());
    //                     print_guest_backtrace(&spt.page_table, satp, ctx);
    //                     spt.page_table.map(vpn, ppn, flags);
    //                 }else{
    //                     if let Some(pte) = spt.page_table.translate(vpn) {
    //                         if pte.is_valid(){ spt.page_table.unmap(vpn); }
    //                     }
    //                     rmap.rmap.remove(&PhysPageNum::from(vaddr >> 12));
    //                 }
    //             }else {
    //                 // hdebug!("vpn: {:#x}, ppn: {:#x}, root_ppn: {:#x}, i = {}", vpn, ppn.0, root_ppn.0, i);
    //                 if pte.bits == 0{
    //                     // clean
    //                     // hdebug!("vaddr: {:#x}", vaddr);
    //                 }else{
    //                     unimplemented!()
    //                 }
    //             }
    //         }else{
    //             unimplemented!()
    //         }
            
    //     }
    //  }

    pub fn synchronize_page_table(&mut self, va: usize) {
        let hart_id = self.index;
        let vpn = VirtPageNum::from(va >> 12);
        if let Some(info) = self.shadow_state.shadow_page_tables.spt_by_vpn_mut(vpn) {
            // 找到所修改页表项对应的影子页表
            let spt = &mut info.spt;
            let gpt = &info.gpt;
            let vpns = &mut info.vpns;
            match info.mode {
                PageTableRoot::GVA => {
                    synchronize_kernel_page_table(hart_id, spt, gpt, vpns);
                },
                PageTableRoot::UVA => {
                    let guest_spt_info = self.shadow_state.shadow_page_tables.guest_page_table_mut().unwrap();
                    let guest_spt = &mut guest_spt_info.spt;
                    synchronize_user_page_table(hart_id, spt, guest_spt, gpt, vpns);
                },
                _ => unreachable!()
            }
        }else{
            unimplemented!()
        }
    }

}

