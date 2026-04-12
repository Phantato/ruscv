use alloc::collections::btree_map::BTreeMap;
use bitflags::*;

use crate::memory::PTE_PER_PAGE;

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
    pub fn contains_all(flag: PTEFlags) -> impl FnOnce(&&Self) -> bool {
        move |pte| (pte.flags() & flag) == flag
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

struct PageTableFrame {
    frame: PageFrame,
    mc: usize,
}

impl PageTableFrame {
    fn get_pte_array_mut(&self) -> &'static mut [PageTableEntry; PTE_PER_PAGE] {
        unsafe { PhysAddr::from(self.frame.ppn).get_mut().unwrap() }
    }
    fn get(&self, idx: usize) -> &'static PageTableEntry {
        return &self.get_pte_array_mut()[idx];
    }
    fn map(&mut self, idx: usize, ppn: PhysPageNum, mode: PTEFlags) {
        self.mc += 1;
        let pte = &mut self.get_pte_array_mut()[idx];
        assert!(!pte.is_valid(), "ppn {:?} is mapped before mapping", ppn);
        *pte = PageTableEntry::new(ppn, mode | PTEFlags::V);
    }
    fn unmap(&mut self, idx: usize) -> bool {
        self.get_pte_array_mut()[idx] = PageTableEntry::empty();
        self.mc -= 1;
        self.mc == 0
    }
}

impl From<PageFrame> for PageTableFrame {
    fn from(frame: PageFrame) -> Self {
        Self { frame, mc: 0 }
    }
}

pub struct PageTable {
    root: PhysPageNum,
    frames: BTreeMap<PhysPageNum, PageTableFrame>,
}

impl PageTable {
    pub fn new() -> Self {
        let root_frame = frame_alloc().unwrap();
        let root = root_frame.ppn;
        let mut frames = BTreeMap::new();
        frames.insert(root, root_frame.into());
        Self { root, frames }
    }
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, mode: PTEFlags) {
        let idxs = vpn.indexes();
        self.get_page_table_frame_or_create(idxs)
            .map(idxs[3], ppn, mode)
    }
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let idxs = vpn.indexes();
        self.recycle_sub(vpn, idxs, self.root, 0);
    }
    fn recycle_sub(
        &mut self,
        vpn: VirtPageNum,
        idxs: [usize; 4],
        ppn: PhysPageNum,
        iidx: usize,
    ) -> bool {
        if iidx >= idxs.len() {
            return true;
        }
        let idx = idxs[iidx];
        let sub_pte = self.frames.get(&ppn).unwrap().get(idx);
        assert!(
            sub_pte.is_valid(),
            "vpn {:?} is invalid before unmapping",
            vpn.0
        );
        if self.recycle_sub(vpn, idxs, sub_pte.ppn(), iidx + 1) {
            self.frames.remove(&sub_pte.ppn());
            self.frames.get_mut(&ppn).unwrap().unmap(idx)
        } else {
            false
        }
    }

    pub fn token(&self) -> usize {
        9usize << 60 | self.root.0
    }

    pub fn translate(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.get_pte(va.floor())
            .filter(|pte| pte.is_valid())
            .map(|pte| PhysAddr::from(pte.ppn()) + va.offset())
    }
    pub fn translate_user(&self, va: VirtAddr, mode: PTEFlags) -> Result<PhysAddr, ()> {
        self.get_pte(va.floor())
            .filter(PageTableEntry::contains_all(
                mode | PTEFlags::V | PTEFlags::U,
            ))
            .map(|pte| PhysAddr::from(pte.ppn()) + va.offset())
            .ok_or(())
    }
    fn get_page_table_frame_or_create(&mut self, idxs: [usize; 4]) -> &mut PageTableFrame {
        let mut parent_ppn = self.root;
        for i in 0..3 {
            let idx = idxs[i];
            let current_frame = self.frames.get_mut(&parent_ppn).unwrap();
            let pte = current_frame.get(idx);
            parent_ppn = if pte.is_valid() {
                pte.ppn()
            } else {
                let frame = frame_alloc().unwrap();
                let ppn = frame.ppn;
                current_frame.map(idxs[i], ppn, PTEFlags::V);
                self.frames.insert(ppn, frame.into());
                ppn
            };
        }
        self.frames.get_mut(&parent_ppn).unwrap()
    }
    pub fn get_pte(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
        let idxs = vpn.indexes();
        let (mut pte, mut ppn) = (None, self.root);
        for idx in idxs {
            pte = self
                .frames
                .get(&ppn)
                .map(|frame| frame.get(idx))
                .filter(|pte| pte.is_valid());
            ppn = pte?.ppn();
        }
        pte
    }
}
