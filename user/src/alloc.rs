//TODO: add a global_allocator here.

use core::alloc::GlobalAlloc;

#[global_allocator]
static MEMORY_MANAGER: MemoryManager = unsafe { MemoryManager{}};

 struct MemoryManager {}

unsafe impl GlobalAlloc for MemoryManager {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        unimplemented!();
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        unimplemented!();
    }
}
