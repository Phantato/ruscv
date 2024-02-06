#![allow(unused)]
use alloc::{collections::VecDeque, sync::Arc};

use crate::{
    memory::{
        address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
        kernel_stack_position,
        memory_set::{MemorySet, SegmentPermission},
        KERNEL_SPACE, TRAMPOLINE, TRAP_CONTEXT,
    },
    print, println,
    sbi::{self, shutdown},
    sync::UPSafeCell,
    trace,
    trap::{self, TrapCtx},
};
use core::{arch::asm, fmt};

const MAX_APP_NUM: usize = 16;
const APP_SIZE_LIMIT: usize = 0x40000;

lazy_static::lazy_static! {
    static ref APP_MANAGER: UPSafeCell<TaskManager> = unsafe {
        UPSafeCell::new(TaskManager::new())
    };
}

pub struct ProcessControlBlock {
    trap_ctx_addr: PhysAddr,
    mem_set: MemorySet,
}

impl ProcessControlBlock {
    pub fn trap_ctx(&self) -> &'static mut TrapCtx {
        unsafe { self.trap_ctx_addr.get_mut().unwrap() }
    }
    pub fn satp(&self) -> usize {
        self.mem_set.token()
    }
}

// impl fmt::Display for TaskControlBlock {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(
//             f,
//             "\t[prog no.{}]:{:?}      \tstart from:{:#x},\tlen:{:#x}",
//             self.seq_no,
//             unsafe { ffi::CStr::from_ptr(self.name as *const _) },
//             self.start,
//             self.len
//         )
//     }
// }

impl ProcessControlBlock {
    fn from_elf(elf: &[u8], task_id: usize) -> Self {
        let (mem_set, sp, entry) = MemorySet::from_elf(elf);
        let trap_ctx_addr: PhysAddr = mem_set
            .translate(VirtAddr::from(TRAP_CONTEXT).floor())
            .expect("TRAP_CONTEXT should be mapped")
            .ppn()
            .into();
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(task_id);
        {
            KERNEL_SPACE.get_mut().push_empty_seg(
                kernel_stack_bottom.into(),
                kernel_stack_top.into(),
                SegmentPermission::R, // | SegmentPermission::W,
            );
        }
        unsafe {
            *trap_ctx_addr.get_mut().unwrap() =
                TrapCtx::new_app(entry, sp, KERNEL_SPACE.get().token(), kernel_stack_top);
        }
        ProcessControlBlock {
            trap_ctx_addr,
            mem_set,
        }
    }

    pub fn translate(&self, va: VirtAddr) -> Option<PhysAddr> {
        let diff = va.0 - VirtAddr::from(va.floor()).0;
        self.mem_set
            .translate(va.floor())
            .map(|pte| PhysAddr::from(pte.ppn()) + diff)
    }
}

struct TaskManager {
    num: usize,
    next: usize,
    load: VecDeque<Arc<ProcessControlBlock>>,
}

impl fmt::Display for TaskManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "num: {}\r", self.num)?;
        writeln!(f, "current: {}\r", self.next - 1)
    }
}

impl TaskManager {
    pub unsafe fn new() -> Self {
        extern "C" {
            fn _num_app();
        }
        let num_ptr = _num_app as usize as *const usize;
        let num = num_ptr.read_volatile();
        trace!("task num: {}", num);
        let mut load = VecDeque::new();
        let app_start = unsafe { core::slice::from_raw_parts(num_ptr.add(1), num + 1) };
        for i in 0..num {
            trace!("load task {}", i);
            load.push_back(
                ProcessControlBlock::from_elf(
                    core::slice::from_raw_parts(
                        app_start[i] as *const u8,
                        app_start[i + 1] - app_start[i],
                    ),
                    i,
                )
                .into(),
            )
        }
        trace!("all task loaded");
        Self { num, load, next: 0 }
    }
    fn get_next_task(&mut self) -> Option<Arc<ProcessControlBlock>> {
        if self.next == self.load.len() {
            None
        } else {
            self.next += 1;
            Some(self.load[self.next - 1].clone())
        }
    }
    fn get_current_app(&self) -> Option<Arc<ProcessControlBlock>> {
        if self.next == 0 {
            None
        } else {
            Some(self.load[self.next - 1].clone())
        }
    }
}

pub fn run_next() -> ! {
    let task;
    {
        task = APP_MANAGER.get_mut().get_next_task()
    }
    if let Some(task) = task {
        restore_to_app(&task)
    } else {
        shutdown(false)
    }
}

pub fn restore_to_app(app: &ProcessControlBlock) -> ! {
    trap::init();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    trace!("restore to app");
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") TRAP_CONTEXT,
            in("a1") app.satp(),
            options(noreturn)
        );
    }
    unreachable!()
}

pub fn get_current_app() -> Option<Arc<ProcessControlBlock>> {
    APP_MANAGER.get().get_current_app()
}
