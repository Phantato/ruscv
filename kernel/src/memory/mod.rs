mod free_list;
mod heap;

use crate::{
    kernel_address::{eheap, sheap},
    println,
    sync::UPSafeCell,
};
use core::{alloc::GlobalAlloc, ptr::NonNull};
use heap::Heap;

struct MemoryManager {
    inner: UPSafeCell<Heap<32>>,
}

#[global_allocator]
static MEMORY_MANAGER: MemoryManager = unsafe { MemoryManager::empty() };
impl MemoryManager {
    const unsafe fn empty() -> Self {
        Self {
            inner: UPSafeCell::new(Heap::empty()),
        }
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

pub fn init_heap(start: usize, end: usize) {
    unsafe { MEMORY_MANAGER.inner.get_mut().add(start, end) }
}

#[allow(unused)]
pub fn heap_test() {
    extern crate alloc;
    use alloc::boxed::Box;
    use alloc::vec::Vec;
    let heap_range = sheap as usize..eheap as usize;
    let a = Box::new(5);
    assert_eq!(*a, 5);
    assert!(heap_range.contains(&(a.as_ref() as *const _ as usize)));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
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
