use crate::{memory::{address::PhysAddr, memory_set::{MemorySet, SegmentPermission}, *}, process::*, sync::UPSafeCell};

lazy_static::lazy_static! {
    static ref PID_ALLOCATOR: UPSafeCell<PIDAllocator> = unsafe {
        UPSafeCell::new(PIDAllocator::new())
    };
}

struct PIDAllocator {
    next: usize,
    recycled: Vec<usize>,
}

impl PIDAllocator {
    fn new() -> Self {
        Self {
            next: 1,
            recycled: vec![],
        }
    }
    fn alloc(&mut self) -> PID {
        if let Some(id) = self.recycled.pop() {
            PID(id)
        } else {
            self.next += 1;
            PID(self.next - 1)
        }
    }
    fn dealloc(&mut self, id: usize) {
        self.recycled.push(id);
    }
}

struct PID(usize);

impl Drop for PID {
    fn drop(&mut self) {
        PID_ALLOCATOR.get_mut().dealloc(self.0);
    }
}

pub struct ProcessControlBlock {
    pid: PID,
    pub(super) inner: UPSafeCell<ProcessControlBlockInner>,
}
// TODO: remove all these pub(super)
pub(super) struct ProcessControlBlockInner {
    pub(super) status: ProcessStatus,
    pub(super) switch_ctx: SwitchCtx,
    pub(super) trap_ctx_addr: PhysAddr,
    pub(super) mem_set: MemorySet,
}

impl ProcessControlBlock {
    pub fn pid(&self) -> usize {
        self.pid.0
    }
    pub(super) fn trap_ctx(&self) -> &'static mut TrapCtx {
        unsafe { self.inner.get().trap_ctx_addr.get_mut().unwrap() }
    }
    pub(super) fn satp(&self) -> usize {
        self.inner.get().mem_set.token()
    }
    pub(super) fn from_elf(elf: &[u8]) -> Self {
        let pid = PID_ALLOCATOR.get_mut().alloc();
        let id = pid.0;
        Self {
            pid,
            inner: unsafe { UPSafeCell::new(ProcessControlBlockInner::from_elf(elf, id)) },
        }
    }
    pub fn translate(&self, va: VirtAddr, expect: PTEFlags) -> Result<PhysAddr, ()> {
        self.inner.get().mem_set.translate_user(va, expect)
    }
}

impl ProcessControlBlockInner {
    fn from_elf(elf: &[u8], task_id: usize) -> Self {
        let (mem_set, sp, entry) = MemorySet::from_elf(elf);
        let trap_ctx_addr = mem_set.trap_ctx().expect("TRAP_CONTEXT should be mapped");
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(task_id);
        {
            KERNEL_SPACE.get_mut().push_empty_seg(
                kernel_stack_bottom.into(),
                kernel_stack_top.into(),
                SegmentPermission::R | SegmentPermission::W,
            );
        }
        unsafe {
            *trap_ctx_addr.get_mut().unwrap() =
                TrapCtx::new_app(entry, sp, KERNEL_SPACE.get().token(), kernel_stack_top);
        }
        let switch_ctx = SwitchCtx::restore(kernel_stack_top);
        ProcessControlBlockInner {
            status: ProcessStatus::Ready,
            switch_ctx,
            trap_ctx_addr,
            mem_set,
        }
    }
}
