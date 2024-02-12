extern crate alloc;

use core::{arch::asm, slice};

use alloc::{collections::BTreeMap, vec::Vec};
use bitflags::bitflags;
use riscv::register::satp;

use crate::{
    info,
    kernel_address::{
        bstack, ebss, edata, ekernel, erodata, etext, sbss, sdata, srodata, stext, strampoline,
        tstack,
    },
    memory::{KERNEL_SPACE, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE},
    trace,
};

use super::{
    address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, PageFrame},
    page_table::{PTEFlags, PageTable},
    MEMORY_END, PAGE_SIZE,
};

#[allow(unused)]
pub enum SegmentType {
    Framed,
    Linear(usize),
}

bitflags! {
    pub struct SegmentPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

struct Segment {
    start: VirtPageNum,
    end: VirtPageNum,
    data_frames: BTreeMap<VirtPageNum, PageFrame>,
    seg_type: SegmentType,
    seg_perm: SegmentPermission,
}

impl Segment {
    fn new(
        start: VirtAddr,
        end: VirtAddr,
        seg_type: SegmentType,
        seg_perm: SegmentPermission,
    ) -> Self {
        let start = start.floor();
        let end = end.ceil();
        Self {
            start,
            end,
            seg_type,
            seg_perm,
            data_frames: BTreeMap::new(),
        }
    }

    fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.start..self.end {
            self.map_one(page_table, vpn)
        }
    }
    fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.start..self.end {
            self.unmap_one(page_table, vpn)
        }
    }
    fn copy_data(&mut self, data: &[u8]) {
        let mut start: usize = 0;
        let len = data.len();
        for ref vpn in self.start..self.end {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut self
                .data_frames
                .get_mut(vpn)
                .expect(&format!("vpn 0x{:x} not found", vpn.0))
                .get_bytes_array_mut()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
        }
    }
    fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let flags = PTEFlags::from_bits(self.seg_perm.bits).unwrap();
        let ppn = match self.seg_type {
            SegmentType::Framed => {
                let frame = frame_alloc().unwrap();
                let ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
                ppn
            }
            SegmentType::Linear(offset) => PhysPageNum(vpn.0 - offset),
        };
        page_table.map(vpn, ppn, flags);
    }
    fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.seg_type {
            SegmentType::Framed => {
                self.data_frames.remove(&vpn);
            }
            SegmentType::Linear(_) => {}
        }
        page_table.unmap(vpn)
    }
}

pub struct MemorySet {
    pub page_table: PageTable,
    segments: Vec<Segment>,
}

impl MemorySet {
    // /// Include sections in elf and trampoline and TrapContext and user stack,
    // /// also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new();
        // map trampoline
        memory_set.map_trampoline(
            KERNEL_SPACE
                .get()
                .translate(VirtAddr::from(strampoline as usize))
                .expect("text seg should be mapped!")
                .floor(),
        );
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = SegmentPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= SegmentPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= SegmentPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= SegmentPermission::X;
                }
                let seg = Segment::new(start_va, end_va, SegmentType::Framed, map_perm);
                max_end_vpn = seg.end;
                memory_set.push(
                    seg,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
                trace!(
                    "map [0x{:x}, 0x{:x}) to [0x{:x}, 0x{:x})",
                    start_va.0,
                    end_va.0,
                    memory_set.translate(start_va).unwrap().floor().0,
                    memory_set.translate(end_va).unwrap().ceil().0,
                )
            }
        }
        // map user stack with U flags
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // guard page
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            Segment::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                SegmentType::Framed,
                SegmentPermission::R | SegmentPermission::W | SegmentPermission::U,
            ),
            None,
        );
        // map TrapContext
        memory_set.push(
            Segment::new(
                TRAP_CONTEXT.into(),
                TRAMPOLINE.into(),
                SegmentType::Framed,
                SegmentPermission::R | SegmentPermission::W,
            ),
            None,
        );
        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }

    pub fn new_kernel() -> Self {
        let mut kernel = MemorySet::new();
        kernel.push(
            Segment::new(
                (stext as usize).into(),
                (etext as usize).into(),
                SegmentType::Framed,
                SegmentPermission::X,
            ),
            Some(unsafe {
                slice::from_raw_parts(stext as *const u8, etext as usize - stext as usize)
            }),
        );
        kernel.push(
            Segment::new(
                (srodata as usize).into(),
                (erodata as usize).into(),
                SegmentType::Framed,
                SegmentPermission::R,
            ),
            Some(unsafe {
                slice::from_raw_parts(srodata as *const u8, erodata as usize - srodata as usize)
            }),
        );
        kernel.push(
            Segment::new(
                (sbss as usize).into(),
                (ebss as usize).into(),
                SegmentType::Framed,
                SegmentPermission::R | SegmentPermission::W,
            ),
            Some(unsafe {
                slice::from_raw_parts(sbss as *const u8, ebss as usize - sbss as usize)
            }),
        );
        kernel.push(
            Segment::new(
                (bstack as usize).into(),
                (tstack as usize).into(),
                SegmentType::Framed,
                SegmentPermission::R | SegmentPermission::W,
            ),
            Some(unsafe {
                slice::from_raw_parts(bstack as *const u8, tstack as usize - bstack as usize)
            }),
        );
        kernel.push(
            Segment::new(
                (ekernel as usize).into(),
                (MEMORY_END as usize).into(),
                SegmentType::Linear(0),
                SegmentPermission::R | SegmentPermission::W,
            ),
            None,
        );
        // trampoline page need to be manually configured.
        kernel.map_trampoline(
            kernel
                .translate(VirtAddr::from(strampoline as usize))
                .unwrap()
                .floor(),
        );

        kernel.push(
            Segment::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                SegmentType::Framed,
                SegmentPermission::R | SegmentPermission::W,
            ),
            Some(unsafe {
                slice::from_raw_parts(sdata as *const u8, edata as usize - sdata as usize)
            }),
        );

        kernel.activate();
        kernel
    }

    pub fn push_empty_seg(&mut self, start: VirtAddr, end: VirtAddr, seg_perm: SegmentPermission) {
        self.push(
            Segment::new(start, end, SegmentType::Framed, seg_perm),
            None,
        );
    }

    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    pub fn translate(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.page_table.translate(va)
    }

    fn map_trampoline(&mut self, trampoline_ppn: PhysPageNum) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).floor(),
            trampoline_ppn,
            PTEFlags::R | PTEFlags::X,
        );
    }

    fn new() -> Self {
        Self {
            page_table: PageTable::new(),
            segments: Vec::new(),
        }
    }

    fn push(&mut self, mut segment: Segment, data: Option<&[u8]>) {
        segment.map(&mut self.page_table);
        if let Some(data) = data {
            segment.copy_data(data);
        }
        self.segments.push(segment);
    }

    fn activate(&self) {
        let satp = self.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.get_mut();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space
            .page_table
            .get_pte(mid_text.floor())
            .unwrap()
            .writable(),
        false
    );
    assert_eq!(
        kernel_space
            .page_table
            .get_pte(mid_rodata.floor())
            .unwrap()
            .writable(),
        false,
    );
    assert_eq!(
        kernel_space
            .page_table
            .get_pte(mid_data.floor())
            .unwrap()
            .executable(),
        false,
    );
    info!("remap_test passed!");
}
