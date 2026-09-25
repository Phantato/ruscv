use crate::{
    memory::{
        address::PhysAddr,
        memory_set::MemorySet,
        *,
    },
    process::{kernel_stack::KernelStack, *},
    sync::UPSafeCell,
};

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
    fn alloc(&mut self) -> PIDSlot {
        if let Some(id) = self.recycled.pop() {
            PIDSlot(id)
        } else {
            self.next += 1;
            PIDSlot(self.next - 1)
        }
    }
    fn dealloc(&mut self, id: usize) {
        self.recycled.push(id);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PID(pub usize);

impl core::fmt::Display for PID{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

struct PIDSlot(usize);

impl PIDSlot {
    fn pid(&self) -> PID {
        PID(self.0)
    }
}

impl Drop for PIDSlot {
    fn drop(&mut self) {
        PID_ALLOCATOR.get_mut().dealloc(self.0);
    }
}

pub struct ProcessControlBlock {
    pub(super) inner: UPSafeCell<ProcessControlBlockInner>,
    // this should be recycled at the last.
    slot: PIDSlot,
}
// TODO: remove all these pub(super)
pub(super) struct ProcessControlBlockInner {
    pub(super) status: ProcessStatus,
    pub(super) switch_ctx: SwitchCtx,
    pub(super) trap_ctx_addr: PhysAddr,
    pub(super) mem_set: MemorySet,
    pub(super) kernel_stack: KernelStack,
}

impl ProcessControlBlock {
    pub fn pid(&self) -> PID {
        self.slot.pid()
    }
    pub(super) fn trap_ctx(&self) -> &'static mut TrapCtx {
        unsafe { self.inner.get().trap_ctx_addr.get_mut().unwrap() }
    }
    pub(super) fn satp(&self) -> usize {
        self.inner.get().mem_set.token()
    }
    pub(super) fn from_elf(elf: &[u8]) -> Self {
        let slot = PID_ALLOCATOR.get_mut().alloc();
        let pid = slot.pid();
        Self {
            slot,
            inner: unsafe { UPSafeCell::new(ProcessControlBlockInner::from_elf(elf, pid)) },
        }
    }
    pub fn translate(&self, va: VirtAddr, expect: PTEFlags) -> Result<PhysAddr, ()> {
        self.inner.get().mem_set.translate_user(va, expect)
    }
}

impl ProcessControlBlockInner {
    fn from_elf(elf: &[u8], pid: PID) -> Self {
        let (mem_set, sp, entry) = MemorySet::from_elf(elf);
        let trap_ctx_addr = mem_set.trap_ctx().expect("TRAP_CONTEXT should be mapped");
        let kernel_stack = KernelStack::new(pid.0);
        unsafe {
            *trap_ctx_addr.get_mut().unwrap() =
                TrapCtx::new_app(entry, sp, KERNEL_SPACE.get().token(), kernel_stack.top());
        }
        let switch_ctx = SwitchCtx::restore(kernel_stack.top());
        ProcessControlBlockInner {
            status: ProcessStatus::Ready,
            switch_ctx,
            trap_ctx_addr,
            mem_set,
            kernel_stack,
        }
    }
}
