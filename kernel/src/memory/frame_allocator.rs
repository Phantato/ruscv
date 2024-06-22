use super::{
    address::{PhysAddr, PhysPageNum},
    page_table::PageTableEntry,
    MEMORY_END, PAGE_SIZE, PTE_PER_PAGE,
};
use crate::{
    info,
    kernel_address::{ekernel, skernel},
    sync::UPSafeCell,
    trace,
};
use alloc::vec::Vec;

pub struct PageFrame {
    pub(crate) ppn: PhysPageNum,
}

impl PageFrame {
    pub fn get_pte_array_mut(&self) -> Option<&mut [PageTableEntry; PTE_PER_PAGE]> {
        unsafe { PhysAddr::from(self.ppn).get_mut() }
    }
    pub fn get_bytes_array_mut(&self) -> &mut [u8; PAGE_SIZE] {
        unsafe { PhysAddr::from(self.ppn).get_mut().unwrap() }
    }
}

impl PageFrame {
    fn new(ppn: PhysPageNum) -> Self {
        Self { ppn }
    }
}

impl Drop for PageFrame {
    fn drop(&mut self) {
        FRAME_ALLOCATOR.get_mut().dealloc(self.ppn)
    }
}

static FRAME_ALLOCATOR: UPSafeCell<StackFrameAllocator> =
    unsafe { UPSafeCell::new(StackFrameAllocator::new()) };

pub trait FrameAllocator {
    fn alloc(&mut self) -> Option<PageFrame>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

pub fn init_frame_allocator() {
    FRAME_ALLOCATOR.get_mut().init(
        PhysAddr::from(ekernel as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

#[allow(unused)]
pub fn recycle_kernel_frames() {
    FRAME_ALLOCATOR.get_mut().add_pages(
        PhysAddr::from(skernel as usize).floor(),
        // kernel is aligned, so floor and ceil will gives the same page number
        PhysAddr::from(ekernel as usize - 1).floor(),
    )
}

struct StackFrameAllocator {
    begin: PhysPageNum,
    end: PhysPageNum,
    recycled: Vec<PhysPageNum>,
}
impl StackFrameAllocator {
    const fn new() -> Self {
        Self {
            begin: PhysPageNum(0),
            end: PhysPageNum(0),
            recycled: vec![],
        }
    }

    pub fn init(&mut self, begin: PhysPageNum, end: PhysPageNum) {
        self.begin = begin;
        self.end = end;
    }
    pub fn add_pages(&mut self, begin: PhysPageNum, end: PhysPageNum) {
        for ppn in begin..end {
            self.recycled.push(ppn)
        }
    }
}
impl FrameAllocator for StackFrameAllocator {
    fn alloc(&mut self) -> Option<PageFrame> {
        let frame = if let Some(ppn) = self.recycled.pop() {
            Some(PageFrame::new(ppn))
        } else {
            if self.begin != self.end {
                self.begin.0 += 1;
                Some(PageFrame::new((self.begin.0 - 1).into()))
            } else {
                None
            }
        };
        frame.and_then(|frame| {
            frame.get_bytes_array_mut().fill(0);
            Some(frame)
        })
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        if ppn >= self.begin || self.recycled.iter().find(|r| ppn == **r).is_some() {
            panic!("deallocing page {} is not allocated yes", ppn.0)
        }
        self.recycled.push(ppn);
        trace!("{} recycled!", ppn.0);
    }
}

pub fn frame_alloc() -> Option<PageFrame> {
    FRAME_ALLOCATOR.get_mut().alloc()
}

#[allow(unused)]
pub fn frame_allocator_test() {
    {
        let mut v: Vec<PageFrame> = vec![];
        for i in 0..5 {
            let frame = frame_alloc().unwrap();
            v.push(frame);
        }
        v.clear();
        for i in 0..5 {
            let frame = frame_alloc().unwrap();
            v.push(frame);
        }
    }
    info!("frame_allocator_test passed!");
}
