extern crate alloc;

use core::{arch::asm, slice};

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};
use bitflags::bitflags;
use riscv::register::satp;

use crate::{
    kernel_address::{
        bstack, ebss, edata, erodata, etext, sbss, sdata, srodata, stext, strampoline, tstack,
    }, //strampoline},
    // memory::PAGE_SIZE_BITS,
    println,
    sync::UPSafeCell,
};

use super::{
    address::{PhysPageNum, VirtAddr, VirtPageNum},
    frame_allocator::{frame_alloc, PageFrame},
    page_table::{PTEFlags, PageTable},
    MEMORY_END, PAGE_SIZE,
};

lazy_static::lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;

#[allow(unused)]
pub enum SegmentType {
    Framed,
    Identical,
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

#[allow(unused)]
impl Segment {
    pub fn new(
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

    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.start..self.end {
            self.map_one(page_table, vpn)
        }
    }
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.start..self.end {
            self.unmap_one(page_table, vpn)
        }
    }
    pub fn copy_data(&mut self, data: &[u8]) {
        let mut start: usize = 0;
        let len = data.len();
        for ref vpn in self.start..self.end {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst =
                &mut self.data_frames.get_mut(vpn).unwrap().get_bytes_array_mut()[..src.len()];
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
            SegmentType::Identical => PhysPageNum(vpn.0),
        };
        page_table.map(vpn, ppn, flags);
    }
    fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.seg_type {
            SegmentType::Framed => {
                self.data_frames.remove(&vpn);
                page_table.unmap(vpn)
            }
            SegmentType::Identical => {}
        }
    }
}

pub struct MemorySet {
    page_table: PageTable,
    segments: Vec<Segment>,
}

#[allow(unused)]
impl MemorySet {
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
    // Assume that no conflicts.
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: SegmentPermission,
    ) {
        self.push(
            Segment::new(start_va, end_va, SegmentType::Framed, permission),
            None,
        );
    }

    pub fn map_trampoline(&mut self, trampoline_ppn: PhysPageNum) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).floor(),
            trampoline_ppn,
            PTEFlags::X,
        )
    }

    pub fn new_kernel() -> Self {
        let mut kernel = MemorySet::new();
        kernel.push(
            Segment::new(
                (stext as usize).into(),
                (etext as usize).into(),
                SegmentType::Framed,
                SegmentPermission::R | SegmentPermission::X,
            ),
            // None,
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
            // None,
            Some(unsafe {
                slice::from_raw_parts(srodata as *const u8, erodata as usize - srodata as usize)
            }),
        );
        kernel.push(
            Segment::new(
                (sdata as usize).into(),
                (edata as usize).into(),
                SegmentType::Framed,
                SegmentPermission::R | SegmentPermission::W,
            ),
            // None,
            Some(unsafe {
                slice::from_raw_parts(sdata as *const u8, edata as usize - sdata as usize)
            }),
        );
        kernel.push(
            Segment::new(
                (sbss as usize).into(),
                (ebss as usize).into(),
                SegmentType::Framed,
                SegmentPermission::R | SegmentPermission::W,
            ),
            // None,
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
            // None,
            Some(unsafe {
                slice::from_raw_parts(bstack as *const u8, tstack as usize - bstack as usize)
            }),
        );
        kernel.push(
            Segment::new(
                (tstack as usize).into(),
                (MEMORY_END as usize).into(),
                SegmentType::Identical,
                SegmentPermission::R | SegmentPermission::W,
            ),
            None,
        );
        kernel.map_trampoline(
            kernel
                .page_table
                .translate(VirtAddr::from(strampoline as usize).floor())
                .expect("text seg should be mapped!")
                .ppn(),
        );
        kernel.activate();
        kernel
    }

    fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
    // /// Include sections in elf and trampoline and TrapContext and user stack,
    // /// also returns user_sp and entry point.
    // pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize);
}

#[allow(unused)]
pub fn remap_test() {
    println!("remap_test testing!");
    let mut kernel_space = KERNEL_SPACE.get_mut();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_text.floor())
            .unwrap()
            .writable(),
        false
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_rodata.floor())
            .unwrap()
            .writable(),
        false,
    );
    assert_eq!(
        kernel_space
            .page_table
            .translate(mid_data.floor())
            .unwrap()
            .executable(),
        false,
    );
    println!("remap_test passed!");
}
