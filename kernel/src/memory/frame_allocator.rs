use super::{
    address::{PhysAddr, PhysPageNum},
    page_table::PageTableEntry,
    MEMORY_END, PAGE_SIZE, PTE_PER_PAGE,
};
use crate::{
    kernel_address::{ekernel, skernel},
    println,
    sync::UPSafeCell,
};
use alloc::vec::Vec;

pub struct PageFrame {
    pub(super) ppn: PhysPageNum,
}

impl PageFrame {
    pub fn get_pte_array_mut(&self) -> &mut [PageTableEntry] {
        // FIXME: this is not safe when concurrency
        let pa: PhysAddr = self.ppn.into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut PageTableEntry, PTE_PER_PAGE) }
    }
    pub fn get_bytes_array_mut(&self) -> &mut [u8] {
        // FIXME: this is not safe when concurrency
        let pa: PhysAddr = self.ppn.into();
        unsafe { core::slice::from_raw_parts_mut(pa.0 as *mut u8, PAGE_SIZE) }
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
        println!("framed page size: {:x}", (end.0 - begin.0) * PAGE_SIZE)
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
        println!("cap: {}!", self.recycled.capacity());
        self.recycled.push(ppn);
        println!("{} recycled!", ppn.0);
    }
}

pub fn frame_alloc() -> Option<PageFrame> {
    FRAME_ALLOCATOR.get_mut().alloc()
}

#[allow(unused)]
pub fn frame_allocator_test() {
    let mut v: Vec<PageFrame> = vec![];
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame.ppn.0);
        v.push(frame);
    }
    v.clear();
    for i in 0..5 {
        let frame = frame_alloc().unwrap();
        println!("{:?}", frame.ppn.0);
        v.push(frame);
    }
    drop(v);
    println!("frame_allocator_test passed!");
}
