pub mod address;
pub mod frame_allocator;
pub mod memory_set;
mod page_table;

use crate::sync::UPSafeCell;
use alloc::sync::Arc;
use memory_set::MemorySet;

pub use self::{address::VirtAddr, page_table::PageTable};

const PA_WIDTH: usize = 56;
const VA_WIDTH: usize = 48;
const PAGE_SIZE_BITS: usize = 12;
const PTE_SIZE: usize = 8;
const PPN_WIDTH: usize = PA_WIDTH - PAGE_SIZE_BITS;
const VPN_WIDTH: usize = VA_WIDTH - PAGE_SIZE_BITS;
const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;
const PTE_PER_PAGE: usize = PAGE_SIZE / PTE_SIZE;
pub const MEMORY_END: usize = 0x8080_0000;
pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;
pub const USER_STACK_SIZE: usize = 4096 * 2;
pub const KERNEL_STACK_SIZE: usize = 4096;

pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}
lazy_static::lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
        Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

pub fn init() {
    frame_allocator::init_frame_allocator();
    lazy_static::initialize(&KERNEL_SPACE);
    // frame_allocator::recycle_kernel_frames();
}

pub fn test() {
    frame_allocator::frame_allocator_test();
    memory_set::remap_test();
}
