mod kernel_stack;
mod process_control_block;
mod status;
mod manager;
use self::status::ProcessStatus;
use self::manager::PROCESS_MANAGER;
use crate::timer::set_next_trigger;
use crate::{
    error,
    memory::{TRAMPOLINE, TRAP_CONTEXT},
    sbi::shutdown,
    sync::UPSafeCell,
    syscall::{syscall, MAX_MSG_LEN},
    trace,
};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
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

pub use crate::process::process_control_block::ProcessControlBlock;

global_asm!(include_str!("switch.s"));
extern "C" {
    fn __switch(current_task_ctx_ptr: *mut SwitchCtx, next_task_ctx_ptr: *const SwitchCtx);
}

global_asm!(include_str!("trap.s"));

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

pub fn exit_current(exit_code: i32) -> ! {
    PROCESS_MANAGER.mark_current_exited(exit_code);
    PROCESS_MANAGER.run_next_process(PROCESS_MANAGER.get_current_switch_ctx());
    unreachable!("process is exited");
}

pub fn fork_current() -> Result<Arc<ProcessControlBlock>, ()> {
    PROCESS_MANAGER.fork_current()
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
    let pid = get_current_process().pid();
    error!("[kernel] {} pid: {}", hint, pid);
    error!("[kernel] instrument at {:#x}", inst_addr);

    exit_current(128);
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
