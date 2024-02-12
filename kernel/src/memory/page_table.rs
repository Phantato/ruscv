use alloc::vec::Vec;
use bitflags::*;

use super::{
    address::{PhysAddr, PhysPageNum, VirtPageNum},
    frame_allocator::{frame_alloc, PageFrame},
    VirtAddr,
};

bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

pub struct PageTable {
    pub root_frame: PageFrame,
    frames: Option<Vec<PageFrame>>,
}

impl PageTable {
    pub fn new() -> Self {
        let root_frame = frame_alloc().unwrap();
        let frames = Some(vec![]);
        Self { root_frame, frames }
    }
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.get_pte_create(vpn);
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn.0);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        if let Some(pte) = self.get_pte(vpn) {
            assert!(
                pte.is_valid(),
                "vpn {:?} is invalid before unmapping",
                vpn.0
            );
            // FIXME: release physical frame.
            *pte = PageTableEntry::empty();
        }
    }

    pub fn token(&self) -> usize {
        9usize << 60 | self.root_frame.ppn.0
    }

    pub fn translate(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.get_pte(va.floor())
            .map(|pte| PhysAddr::from(pte.ppn()) + va.offset())
    }
    pub fn get_pte_create(&mut self, vpn: VirtPageNum) -> &mut PageTableEntry {
        let idxs = vpn.indexes();
        let mut parent_frame = self.root_frame.get_pte_array_mut().unwrap();
        // once got the 4th page table, directly returns its entry.
        for i in 0..3 {
            let pte = &mut parent_frame[idxs[i]];
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                let ppn = frame.ppn;
                self.frames
                    .as_mut()
                    .expect("create pte on temp PageTable")
                    .push(frame);
                *pte = PageTableEntry::new(ppn, PTEFlags::V);
            }
            parent_frame = unsafe { PhysAddr::from(pte.ppn()).get_mut().unwrap() }
        }
        &mut parent_frame[idxs[3]]
    }
    pub fn get_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut parent_frame = self.root_frame.get_pte_array_mut().unwrap();
        // once got the 4th page table, directly returns its entry.
        for i in 0..3 {
            let pte = &parent_frame[idxs[i]];
            if !pte.is_valid() {
                return None;
            }
            parent_frame = unsafe { PhysAddr::from(pte.ppn()).get_mut().unwrap() }
        }
        Some(&mut parent_frame[idxs[3]])
    }
}

impl From<PhysPageNum> for PageTable {
    fn from(ppn: PhysPageNum) -> Self {
        let root_frame = PageFrame { ppn };
        Self {
            root_frame,
            frames: None,
        }
    }
}
