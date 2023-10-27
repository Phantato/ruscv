use alloc::collections::BTreeMap;
use bitflags::*;

use super::{
    address::{PhysPageNum, VirtPageNum},
    frame_allocator::{frame_alloc, PageFrame},
    // PTE_PER_PAGE,
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
#[allow(unused)]
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
    pub(crate) root_ppn: PhysPageNum,
    frames: BTreeMap<PhysPageNum, PageFrame>,
}

#[allow(unused)]
impl PageTable {
    pub fn new() -> Self {
        let root_frame = frame_alloc().unwrap();
        let mut frames = BTreeMap::new();
        // let mut pte_arr = frame.get_pte_array_mut();
        let root_ppn = root_frame.ppn;
        // pte_arr[PTE_PER_PAGE - 1] = PageTableEntry::new(root_ppn, PTEFlags::R | PTEFlags::W);
        frames.insert(root_ppn, root_frame);
        Self { root_ppn, frames }
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
            *pte = PageTableEntry::empty();
        }
    }
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.get_pte(vpn).map(|pte| pte.clone())
    }
    pub fn token(&self) -> usize {
        9usize << 60 | self.root_ppn.0
    }
    fn get_pte_create(&mut self, vpn: VirtPageNum) -> &mut PageTableEntry {
        let idxs = vpn.indexes();
        let mut parent_ppn = self.root_ppn;
        // once got the 4th page table, directly returns its entry.
        for i in 0..3 {
            let mut pte = self.frames[&parent_ppn].get_pte_array_mut()[idxs[i]];
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                let mut ppn = frame.ppn;
                self.frames.insert(ppn, frame);
                self.frames[&parent_ppn].get_pte_array_mut()[idxs[i]] =
                    PageTableEntry::new(ppn, PTEFlags::V);
                pte = self.frames[&parent_ppn].get_pte_array_mut()[idxs[i]];
            }
            parent_ppn = pte.ppn()
        }
        &mut self.frames[&parent_ppn].get_pte_array_mut()[idxs[3]]
    }
    fn get_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut pte = unsafe { &mut self.frames[&self.root_ppn].get_pte_array_mut()[idxs[0]] };
        // once got the 4th page table, directly returns its entry.
        for i in 0..3 {
            if !pte.is_valid() {
                return None;
            } else {
                pte = unsafe { &mut self.frames[&pte.ppn()].get_pte_array_mut()[idxs[i + 1]] };
            }
        }
        Some(pte)
    }
}
