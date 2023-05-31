mod free_list;
mod heap;

use crate::{println, sync::UPSafeCell};
use core::{alloc::GlobalAlloc, ptr::NonNull};
use heap::Heap;

const KERNEL_HEAP_SIZE: usize = 0x10_0000;

#[global_allocator]
static MEMORY_MANAGER: MemoryManager = unsafe { MemoryManager::empty() };
static KERNEL_HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];
struct MemoryManager {
    inner: UPSafeCell<Heap<20>>,
}

impl MemoryManager {
    const unsafe fn empty() -> Self {
        Self {
            inner: UPSafeCell::new(Heap::empty()),
        }
    }
    unsafe fn add(&self, start: usize, end: usize) {
        self.inner.get_mut().add(start, end)
    }
}

// TODO: For small fraction less than 4K, use buffer pool strategy
unsafe impl GlobalAlloc for MemoryManager {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.inner
            .get_mut()
            .alloc(layout)
            .map_or(0 as *mut u8, |p| p.as_ptr())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.inner
            .get_mut()
            .dealloc(NonNull::new_unchecked(ptr), layout)
    }
}

pub fn init_heap() {
    let start = KERNEL_HEAP.as_ptr() as usize;
    let end = start + KERNEL_HEAP_SIZE;
    unsafe { MEMORY_MANAGER.add(start, end) }
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    let start = KERNEL_HEAP.as_ptr() as usize;
    let end = start + KERNEL_HEAP_SIZE;
    let heap_range = start..end;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(heap_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = vec![];
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    assert!(heap_range.contains(&(v.as_ptr() as usize)));
    drop(v);
    println!("heap_test passed!");
}
