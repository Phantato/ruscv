mod status;
use self::status::ProcessStatus;
use crate::memory::{kernel_stack_position, memory_set::SegmentPermission, KERNEL_SPACE};
use crate::timer::set_next_trigger;
use crate::{
    error,
    memory::{address::PhysAddr, memory_set::MemorySet, VirtAddr, TRAMPOLINE, TRAP_CONTEXT},
    sbi::shutdown,
    sync::UPSafeCell,
    syscall::{syscall, MAX_MSG_LEN},
    trace,
};
use alloc::{collections::VecDeque, sync::Arc};
use core::arch::{asm, global_asm};
use riscv::register::scause::Interrupt;
use riscv::register::sie;
use riscv::register::{
    scause::{self, Exception, Trap},
    sstatus::{self, Sstatus},
    stval, stvec,
    utvec::TrapMode,
};

global_asm!(include_str!("switch.s"));
extern "C" {
    fn __switch(current_task_ctx_ptr: *mut SwitchCtx, next_task_ctx_ptr: *const SwitchCtx);
}

global_asm!(include_str!("trap.s"));

const MAX_APP_NUM: usize = 16;
const APP_SIZE_LIMIT: usize = 0x40000;

lazy_static::lazy_static! {
    static ref PROCESS_MANAGER: ProcessManager = unsafe {
        ProcessManager::new()
    };
}

struct ProcessManager {
    num: usize,
    inner: UPSafeCell<ProcessManagerInner>,
}
struct ProcessManagerInner {
    current: usize,
    load: VecDeque<Arc<ProcessControlBlock>>,
}

impl ProcessManager {
    unsafe fn new() -> Self {
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
                    i,
                    core::slice::from_raw_parts(
                        app_start[i] as *const u8,
                        app_start[i + 1] - app_start[i],
                    ),
                    i,
                )
                .into(),
            )
        }
        let inner = UPSafeCell::new(ProcessManagerInner {
            current: num - 1,
            load,
        });
        trace!("all task loaded");
        Self { num, inner }
    }

    fn start(&self) -> ! {
        self.run_next_process(&mut SwitchCtx::zero() as *mut SwitchCtx);
        unreachable!("unreachable in ProcessManager::start")
    }

    pub fn run_next_process(&self, current_ctx: *mut SwitchCtx) {
        match self.find_next_ready_task() {
            Some(pcb) => {
                let next_ctx = {
                    let mut inner = pcb.inner.get_mut();
                    inner.status = ProcessStatus::Running;
                    &inner.switch_ctx as *const SwitchCtx
                };
                {
                    let mut inner = self.inner.get_mut();
                    inner.current = pcb.pid;
                }
                unsafe { __switch(current_ctx, next_ctx) }
            }
            None => shutdown(false),
        }
    }
    fn find_next_ready_task(&self) -> Option<Arc<ProcessControlBlock>> {
        let inner = self.inner.get_mut();
        let current = inner.current;
        (current + 1..current + self.num + 1)
            .map(|idx| inner.load[idx % self.num].clone())
            .find(|pcb| pcb.inner.get().status == ProcessStatus::Ready)
    }
    fn mark_current_ready(&self) {
        let inner = self.inner.get_mut();
        let mut pcb_inner = inner.load[inner.current].inner.get_mut();
        pcb_inner.status = ProcessStatus::Ready
    }
    fn mark_current_exited(&self) {
        let inner = self.inner.get_mut();
        let mut pcb_inner = inner.load[inner.current].inner.get_mut();
        pcb_inner.status = ProcessStatus::Exited
    }
    pub fn get_current_process(&self) -> Option<Arc<ProcessControlBlock>> {
        let inner: core::cell::Ref<'_, ProcessManagerInner> = self.inner.get();
        if inner.current == self.num {
            None
        } else {
            Some(inner.load[inner.current].clone())
        }
    }
    fn get_current_switch_ctx(&self) -> *mut SwitchCtx {
        let pcb = self.get_current_process().unwrap();
        let mut inner = pcb.inner.get_mut();
        &mut inner.switch_ctx as *mut SwitchCtx
    }
    fn get_current_satp(&self) -> usize {
        let pcb = self.get_current_process().unwrap();
        pcb.satp()
    }
}

pub struct ProcessControlBlock {
    pid: usize,
    inner: UPSafeCell<ProcessControlBlockInner>,
}

struct ProcessControlBlockInner {
    status: ProcessStatus,
    switch_ctx: SwitchCtx,
    trap_ctx_addr: PhysAddr,
    mem_set: MemorySet,
}

impl ProcessControlBlock {
    fn trap_ctx(&self) -> &'static mut TrapCtx {
        unsafe { self.inner.get().trap_ctx_addr.get_mut().unwrap() }
    }
    fn satp(&self) -> usize {
        self.inner.get().mem_set.token()
    }
    fn from_elf(pid: usize, elf: &[u8], task_id: usize) -> Self {
        Self {
            pid,
            inner: unsafe { UPSafeCell::new(ProcessControlBlockInner::from_elf(elf, task_id)) },
        }
    }
    pub fn translate(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.inner.get().mem_set.translate(va)
    }
}

impl ProcessControlBlockInner {
    fn from_elf(elf: &[u8], task_id: usize) -> Self {
        let (mem_set, sp, entry) = MemorySet::from_elf(elf);
        let trap_ctx_addr = mem_set
            .translate(VirtAddr::from(TRAP_CONTEXT))
            .expect("TRAP_CONTEXT should be mapped");
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

#[repr(C)]
pub struct TrapCtx {
    x: [usize; 32],
    sstatus: Sstatus,
    sepc: usize,
    kernel_satp: usize,
    kernel_sp: usize,
    trap_handler: usize,
}

impl TrapCtx {
    pub fn new_app(entry: usize, sp: usize, kernel_satp: usize, kernel_sp: usize) -> Self {
        let trap_handler = trap_from_user as usize;
        let mut sstatus = sstatus::read();
        sstatus.set_spp(sstatus::SPP::User);
        let mut x = [0; 32];
        x[2] = sp;
        Self {
            x,
            sstatus,
            sepc: entry,
            kernel_satp,
            kernel_sp,
            trap_handler,
        }
    }
}

#[repr(C)]
pub struct SwitchCtx {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

impl SwitchCtx {
    pub fn zero() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }
    pub fn restore(sp: usize) -> Self {
        Self {
            ra: restore_to_user as usize,
            s: [0; 12],
            sp,
        }
    }
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(TRAMPOLINE, TrapMode::Direct);
    }
}

pub fn start() -> ! {
    PROCESS_MANAGER.start()
}

pub fn get_current_process() -> Arc<ProcessControlBlock> {
    PROCESS_MANAGER.get_current_process().unwrap()
}

pub fn suspend_current() {
    PROCESS_MANAGER.mark_current_ready();
    PROCESS_MANAGER.run_next_process(PROCESS_MANAGER.get_current_switch_ctx())
}

pub fn exit_current() -> ! {
    PROCESS_MANAGER.mark_current_exited();
    PROCESS_MANAGER.run_next_process(PROCESS_MANAGER.get_current_switch_ctx());
    unreachable!("process is exited");
}

#[no_mangle]
fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

#[no_mangle]
fn trap_from_user() -> ! {
    trace!("trap in");
    set_kernel_trap_entry();
    let mut buf = [0u8; MAX_MSG_LEN];
    let pcb = get_current_process();
    let ctx = pcb.trap_ctx();
    match match scause::read().cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current();
            Ok(())
        }
        Trap::Exception(Exception::UserEnvCall) => {
            trace!("user call id: 0x{:x}", ctx.x[17]);
            let res = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]], &mut buf);
            match res {
                Ok(len) => {
                    ctx.sepc += 4;
                    ctx.x[10] = len as usize;
                    Ok(())
                }
                Err(_) => Err(unsafe { core::str::from_utf8_unchecked(&buf) }),
            }
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            Err("PageFault in application, kernel killed it.")
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            Err("IllegalInstruction in application, kernel killed it.")
        }
        x @ _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", x, stval::read());
        }
    } {
        Ok(_) => restore_to_user(),
        Err(hint) => kernel_fail(ctx.sepc, hint),
    }
}

fn kernel_fail(inst_addr: usize, hint: &str) -> ! {
    let pid = get_current_process().pid;
    error!("[kernel] {} pid: {}", hint, pid);
    error!("[kernel] instrument at {:#x}", inst_addr);

    exit_current();
}

fn restore_to_user() -> ! {
    set_user_trap_entry();
    let satp = PROCESS_MANAGER.get_current_satp();
    extern "C" {
        fn __alltraps();
        fn __restore();
    }
    let restore_va = __restore as usize - __alltraps as usize + TRAMPOLINE;
    trace!("restore to app pc {:#x}", restore_va);
    unsafe {
        asm!(
            "fence.i",
            "jr {restore_va}",
            restore_va = in(reg) restore_va,
            in("a0") TRAP_CONTEXT,
            in("a1") satp,
            options(noreturn)
        );
    }
}

pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}
