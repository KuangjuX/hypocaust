//! Implementation of [`MapArea`] and [`MemorySet`].

use crate::hypervisor::hyp_alloc::{FrameTracker, frame_alloc};
use crate::hypervisor::HYPERVISOR_MEMORY;
use crate::guest::{GuestMemory, GuestPayload, LinuxImage};
use crate::page_table::{PTEFlags, PageTable, PageTableEntry};
use crate::page_table::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use crate::page_table::{StepByOne, VPNRange, PPNRange};
use crate::constants::layout::{
    PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT, MEMORY_END, MMIO,
    SHADOW_PAGE_TABLES_HOST_END, SHADOW_PAGE_TABLES_HOST_START,
};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::arch::asm;
use core::marker::PhantomData;
use riscv::register::satp;

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
    fn sinitrd();
    fn einitrd();
}


/// memory set structure, controls virtual-memory space
pub struct MemorySet<P: PageTable> {
    page_table: P,
    areas: Vec<MapArea<P>>,
}

/// Result of loading a Guest payload into one VM-owned RAM slot.
///
/// PR #51 (`feature/linux-image-loader`) returns the architectural entry point
/// together with its mappings so vCPU construction never assumes `0x80000000`.
pub struct LoadedGuestKernel<P: PageTable> {
    pub memory_set: MemorySet<P>,
    pub entry_gpa: usize,
}

impl<P> MemorySet<P> where P: PageTable {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
        }
    }
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    pub fn page_table(&self) -> &P {
        &self.page_table
    }
    /// Assume that no conflicts.
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(start_va, end_va,  None, None, MapType::Framed, permission),
            None,
        );
    }

    /// 将内存区域 push 到页表中，并映射内存区域
    fn push(&mut self, mut map_area: MapArea<P>, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }
    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }

    /// Without kernel stacks.
    /// 内核虚拟地址映射
    /// 映射了内核代码段和数据段以及跳板页，没有映射内核栈
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        memory_set.push(
            MapArea::new(
                (stext as usize).into(),
                (etext as usize).into(),
                Some((stext as usize).into()),
                Some((etext as usize).into()),
                MapType::Linear,
                MapPermission::R | MapPermission::X,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                Some((srodata as usize).into()),
                Some((erodata as usize).into()),
                MapType::Linear,
                MapPermission::R,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                Some((sdata as usize).into()),
                Some((edata as usize).into()),
                MapType::Linear,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (sbss_with_stack as usize).into(),
                (ebss as usize).into(),
                Some((sbss_with_stack as usize).into()),
                Some((ebss as usize).into()),
                MapType::Linear,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );
        memory_set.push(
            MapArea::new(
                (ekernel as usize).into(),
                MEMORY_END.into(),
                Some((ekernel as usize).into()),
                Some(MEMORY_END.into()),
                MapType::Linear,
                MapPermission::R | MapPermission::W,
            ),
            None,
        );

        // 影子页表映射区域
        memory_set.push(
            MapArea::new(
                VirtAddr::from(SHADOW_PAGE_TABLES_HOST_START),
                VirtAddr::from(SHADOW_PAGE_TABLES_HOST_END),
                Some(PhysAddr::from(SHADOW_PAGE_TABLES_HOST_START)),
                Some(PhysAddr::from(SHADOW_PAGE_TABLES_HOST_END)),
                MapType::Linear,
                MapPermission::R | MapPermission::W
            ),
            None
        );

        for pair in MMIO {
            memory_set.push(
                MapArea::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    Some((*pair).0.into()),
                    Some(((*pair).0 + (*pair).1).into()),
                    MapType::Linear,
                    MapPermission::R | MapPermission::W,
                ),
                None,
            );
        }

        memory_set
    }

    /// 创建用户态的 Guest Kernel 内存空间
    pub fn create_user_guest_kernel(guest_kernel_memory: &Self) -> Self {
        let mut memory_set = Self::new_bare();
        // 代码段：可读可执行
        // 数据段：可读
        // 所有段映射用户空间
        for area in guest_kernel_memory.areas.iter() {
            let mut user_area = area.clone();
            // 添加用户标志
            user_area.map_perm |= MapPermission::U;
            memory_set.push(user_area.clone(), None);
        }
        // 创建跳板页映射
        memory_set.map_trampoline();
        // 映射 Trap Context
        memory_set.push(
            MapArea::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                None,
                None,
                MapType::Framed,
                MapPermission::R | MapPermission::W
            ),
            None,
        );
        
        for pair in MMIO {
            memory_set.push(
                MapArea::new(
                    (*pair).0.into(),
                    ((*pair).0 + (*pair).1).into(),
                    Some((*pair).0.into()),
                    Some(((*pair).0 + (*pair).1).into()),
                    MapType::Linear,
                    MapPermission::R | MapPermission::W | MapPermission::U,
                ),
                None,
            );
        }
        memory_set
    }

    /// Load either the existing xv6-rust ELF payload or a standard little-endian
    /// RISC-V Linux Image into the VM's checked RAM capability.
    pub fn load_guest_kernel(
        payload: GuestPayload<'_>,
        guest_memory: &GuestMemory,
    ) -> LoadedGuestKernel<P> {
        // PR #44 (`fix-bug/map-guest-ram-before-load`) installs this VM's Host
        // RAM mapping before copying its ELF. VM 0 used to work only because
        // it was loaded while the Host still ran with `satp=0`; VM 1 faulted.
        prepare_guest_memory(guest_memory);
        match payload {
            GuestPayload::Elf(bytes) => Self::load_elf_guest_kernel(bytes, guest_memory),
            GuestPayload::LinuxImage(image) => {
                Self::load_linux_guest_kernel(image, guest_memory)
            }
        }
    }

    fn load_elf_guest_kernel(
        guest_kernel_data: &[u8],
        guest_memory: &GuestMemory,
    ) -> LoadedGuestKernel<P> {
        let mut memory_set = Self::new_bare();
        let elf = xmas_elf::ElfFile::new(guest_kernel_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        // PR #36 (`feature/vm-guest-memory`) loads the ELF into the owning VM's
        // Host RAM slot instead of a global VM-0 physical address.
        let mut paddr = guest_memory.host_base() as *mut u8;
        let mut last_paddr = guest_memory.host_base() as *mut u8;
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let segment_start = ph.virtual_addr() as usize;
                let segment_end = ph
                    .virtual_addr()
                    .checked_add(ph.mem_size())
                    .expect("Guest ELF virtual address overflow") as usize;
                let file_start = ph.offset() as usize;
                let file_end = file_start
                    .checked_add(ph.file_size() as usize)
                    .expect("Guest ELF file range overflow");
                let start_va: VirtAddr = segment_start.into();
                let end_va: VirtAddr = segment_end.into();
                assert!(
                    ph.file_size() <= ph.mem_size(),
                    "Guest ELF segment file size exceeds memory size",
                );
                assert!(
                    segment_start >= guest_memory.guest_base()
                        && segment_end <= guest_memory.guest_end(),
                    "Guest ELF segment is outside VM RAM",
                );
                assert!(file_end <= guest_kernel_data.len(), "Guest ELF segment is truncated");
                let mut map_perm = MapPermission { bits: 0 };
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let page_align_size = ph
                    .mem_size()
                    .checked_add((PAGE_SIZE - 1) as u64)
                    .expect("Guest ELF segment size overflow") as usize
                    / PAGE_SIZE
                    * PAGE_SIZE;
                let next_paddr = (paddr as usize)
                    .checked_add(page_align_size)
                    .expect("Guest ELF Host address overflow");
                assert!(
                    next_paddr <= guest_memory.host_end(),
                    "Guest ELF does not fit in VM RAM",
                );
                // 将内存拷贝到对应的物理内存上
                unsafe{
                    core::ptr::copy(
                        guest_kernel_data.as_ptr().add(file_start),
                        paddr,
                        ph.file_size() as usize,
                    );
                    paddr = paddr.add(page_align_size);
                }
                
                let map_area = MapArea::new(
                    start_va, 
                    end_va, 
                    Some(PhysAddr(last_paddr as usize)),
                    Some(PhysAddr(paddr as usize)),
                    MapType::Linear, 
                    map_perm
                );
                last_paddr = paddr;
                memory_set.push(map_area, None);
            }
            
        }
        let offset = paddr as usize - guest_memory.host_base();
        // 映射其他物理内存
        memory_set.push(MapArea::new(
                VirtAddr(offset + guest_memory.guest_base()),
                VirtAddr(guest_memory.guest_end()),
                Some(PhysAddr(paddr as usize)), 
                Some(PhysAddr(guest_memory.host_end())),
                MapType::Linear, 
                MapPermission::R | MapPermission::W
            ),
            None
        );

        let entry_gpa = elf.header.pt2.entry_point() as usize;
        assert!(
            guest_memory.contains_gpa(entry_gpa),
            "Guest ELF entry point is outside VM RAM",
        );
        LoadedGuestKernel {
            memory_set,
            entry_gpa,
        }
    }

    /// PR #51 follows the Linux Image header rather than hard-coding a load
    /// address. With paging disabled, the complete RAM slot is executable and
    /// writable just like supervisor physical memory; Linux installs its own
    /// finer-grained permissions when it enables Sv39.
    fn load_linux_guest_kernel(
        image: LinuxImage<'_>,
        guest_memory: &GuestMemory,
    ) -> LoadedGuestKernel<P> {
        let text_offset = usize::try_from(image.text_offset())
            .expect("Linux Image text offset does not fit the Host address size");
        let image_size = usize::try_from(image.image_size())
            .expect("Linux Image size does not fit the Host address size");
        let occupied_size = image_size.max(image.bytes().len());
        let entry_gpa = guest_memory
            .guest_base()
            .checked_add(text_offset)
            .expect("Linux Image load address overflow");
        let image_hpa = guest_memory
            .translate_range(entry_gpa, occupied_size)
            .expect("Linux Image does not fit in VM RAM");
        unsafe {
            core::ptr::copy_nonoverlapping(
                image.bytes().as_ptr(),
                image_hpa as *mut u8,
                image.bytes().len(),
            );
        }

        let mut memory_set = Self::new_bare();
        memory_set.push(
            MapArea::new(
                VirtAddr::from(guest_memory.guest_base()),
                VirtAddr::from(guest_memory.guest_end()),
                Some(PhysAddr::from(guest_memory.host_base())),
                Some(PhysAddr::from(guest_memory.host_end())),
                MapType::Linear,
                MapPermission::R | MapPermission::W | MapPermission::X,
            ),
            None,
        );
        LoadedGuestKernel {
            memory_set,
            entry_gpa,
        }
    }

    /// 加载客户操作系统
    pub fn hyper_load_guest_kernel(&mut self, guest_kernel_memory: &Self) {
        for area in guest_kernel_memory.areas.iter() {
            // 修改虚拟地址与物理地址相同
            let ppn_range = area.ppn_range.unwrap();
            let start_pa: PhysAddr = ppn_range.get_start().into();
            let end_pa: PhysAddr = ppn_range.get_end().into();
            let start_va: usize = start_pa.into();
            let end_va: usize= end_pa.into();
            let new_area = MapArea::new(
                start_va.into(), 
                end_va.into(), 
                Some(start_pa),
                Some(end_pa), 
                area.map_type, 
                area.map_perm
            );
            self.push(new_area, None);
        }
    }


    /// 激活根页表
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
    
    /// 将虚拟页号翻译成页表项
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
}

/// map area structure, controls a contiguous piece of virtual memory
#[derive(Clone)]
pub struct MapArea<P: PageTable> {
    pub vpn_range: VPNRange,
    pub ppn_range: Option<PPNRange>,
    pub data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    pub map_type: MapType,
    pub map_perm: MapPermission,
    _marker: PhantomData<P>
}

impl<P> MapArea<P> where P: PageTable {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        start_pa: Option<PhysAddr>,
        end_pa: Option<PhysAddr>,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        if let (Some(start_pa), Some(end_pa)) = (start_pa, end_pa) {
            let start_ppn = start_pa.floor();
            let end_ppn = end_pa.ceil();
            return Self {
                vpn_range: VPNRange::new(start_vpn, end_vpn),
                ppn_range: Some(PPNRange::new(start_ppn, end_ppn)),
                data_frames: BTreeMap::new(),
                map_type,
                map_perm,
                _marker: PhantomData
            }
        }
        Self{
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            ppn_range: None,
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
            _marker: PhantomData
        }
    }
    pub fn map_one(&mut self, page_table: &mut P, vpn: VirtPageNum, ppn_: Option<PhysPageNum>) {
        let ppn: PhysPageNum;
        match self.map_type {
            // 线性映射
            MapType::Linear => {
                ppn = ppn_.unwrap();
            },
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }
    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut P, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }
    pub fn map(&mut self, page_table: &mut P) {
        let vpn_range = self.vpn_range;
        if let Some(ppn_range) = self.ppn_range {
            let ppn_start: usize = ppn_range.get_start().into();
            let ppn_end: usize = ppn_range.get_end().into();
            let vpn_start: usize = vpn_range.get_start().into();
            let vpn_end: usize = vpn_range.get_end().into();
            assert_eq!(ppn_end - ppn_start, vpn_end - vpn_start);
            let mut ppn = ppn_range.get_start();
            let mut vpn = vpn_range.get_start();
            loop {
                self.map_one(page_table, vpn, Some(ppn));
                ppn.step();
                vpn.step();
                if ppn == ppn_range.get_end() && vpn == vpn_range.get_end() {
                    break;
                }
            }
        }else{
            for vpn in self.vpn_range {
                self.map_one(page_table, vpn, None)
            }
        }
    }
    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut P) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut P, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }

}

#[derive(Copy, Clone, PartialEq, Debug)]
/// map type for memory set: identical or framed
pub enum MapType {
    Framed,
    Linear
}

bitflags! {
    /// map permission corresponding to that in pte: `R W X U`
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = HYPERVISOR_MEMORY.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();

    assert!(!kernel_space
        .page_table
        .translate(mid_text.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_rodata.floor())
        .unwrap()
        .writable(),);
    assert!(!kernel_space
        .page_table
        .translate(mid_data.floor())
        .unwrap()
        .executable(),);
    // 测试 guest ketnel
    hdebug!("remap test passed!");
}

#[allow(unused)]
pub fn guest_kernel_test(guest_memory: &GuestMemory) {
    let mut kernel_space = HYPERVISOR_MEMORY.exclusive_access();

    let guest_kernel_text: VirtAddr = guest_memory.host_base().into();

    let mapping = kernel_space
        .page_table
        .translate(guest_kernel_text.floor())
        .unwrap();
    // PR #44 maps Guest RAM as Host data, never Host executable code. Guest
    // execution permission remains confined to the vCPU's own page tables.
    assert!(!mapping.executable());
    assert!(mapping.readable());
    // 尝试读数据
    unsafe{
        core::ptr::read(guest_memory.host_base() as *const u32);
    }
    // 测试 guest ketnel
    hdebug!("guest kernel test passed!");
}

/// PR #45 (`feature/multi-guest-qemu`) maps Host-only MMIO apertures discovered
/// from the Host DTB. Guest page tables still expose only their configured GPA.
pub fn map_host_mmio(start: usize, size: usize) {
    assert!(size != 0, "Host MMIO range is empty");
    assert_eq!(start % PAGE_SIZE, 0, "Host MMIO base is not page-aligned");
    assert_eq!(size % PAGE_SIZE, 0, "Host MMIO size is not page-aligned");
    let end = start.checked_add(size).expect("Host MMIO range overflow");
    let mut host_memory = HYPERVISOR_MEMORY.exclusive_access();
    for address in (start..end).step_by(PAGE_SIZE) {
        let vpn = VirtAddr::from(address).floor();
        // PR #46 makes `Some` mean a valid leaf, so this is now a sound
        // already-mapped test rather than a check for allocated page tables.
        if host_memory.page_table.translate(vpn).is_some() {
            continue;
        }
        // Device registers are data and must never become executable in the
        // Host address space merely because a platform reports them.
        host_memory.page_table.map(
            vpn,
            PhysAddr::from(address).floor(),
            PTEFlags::R | PTEFlags::W,
        );
        let installed = host_memory
            .page_table
            .translate(vpn)
            .expect("Host MMIO mapping disappeared after installation");
        assert!(installed.is_valid(), "Host MMIO mapping is invalid");
        assert_eq!(
            installed.ppn(),
            PhysAddr::from(address).floor(),
            "Host MMIO mapping does not preserve the physical aperture",
        );
    }
    host_memory.activate();
}

/// Map one complete VM RAM capability into the Host before any ELF copy, page
/// table walk, or mediated DMA can dereference its Host physical addresses.
fn prepare_guest_memory(guest_memory: &GuestMemory) {
    let mut host_memory = HYPERVISOR_MEMORY.exclusive_access();
    // PR #44 uses R/W without X because Hypocaust accesses Guest RAM only as
    // data; the Guest's page tables independently control Guest execution.
    host_memory.push(
        MapArea::new(
            VirtAddr::from(guest_memory.host_base()),
            VirtAddr::from(guest_memory.host_end()),
            Some(PhysAddr::from(guest_memory.host_base())),
            Some(PhysAddr::from(guest_memory.host_end())),
            MapType::Linear,
            MapPermission::R | MapPermission::W,
        ),
        None,
    );
    // The Host may already have paging enabled while adding a later VM, so
    // publish and fence the new mapping before returning to the ELF loader.
    host_memory.activate();
}
