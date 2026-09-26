mod kernel_stack;
mod manager;
mod process_control_block;
mod status;
use self::manager::PROCESS_MANAGER;
pub use self::status::ProcessStatus;
use crate::process::process_control_block::PID;
use crate::timer::set_next_trigger;
use crate::{
    error,
    memory::{TRAMPOLINE, TRAP_CONTEXT},
    syscall::{syscall, MAX_MSG_LEN},
    trace,
};
use alloc::sync::Arc;
use alloc::vec::Vec;
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

pub fn get_current_process() -> Option<Arc<ProcessControlBlock>> {
    PROCESS_MANAGER.get_current_process()
}

pub fn suspend_current() {
    PROCESS_MANAGER.mark_current_ready();
    PROCESS_MANAGER.run_next_process()
}

pub fn exit_current(exit_code: i32) -> ! {
    PROCESS_MANAGER.mark_current_exited(exit_code);
    PROCESS_MANAGER.run_next_process();
    unreachable!("process is exited");
}

pub fn fork_current() -> Result<Arc<ProcessControlBlock>, ()> {
    PROCESS_MANAGER.fork_current()
}
pub fn exec(elf: &[u8]) -> Result<(), ()> {
    PROCESS_MANAGER.exec(elf)
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

    let ctx = {
        let pcb = get_current_process().unwrap();
        pcb.trap_ctx()
    };
    match match scause::read().cause() {
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            set_next_trigger();
            suspend_current();
            Ok(())
        }
        Trap::Exception(Exception::UserEnvCall) => {
            trace!("user call id: 0x{:x}", ctx.x[17]);
            let res = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]], &mut buf);
            // we need rebuild context here since syscall might replace it.
            let ctx = {
                let pcb = get_current_process().unwrap();
                pcb.trap_ctx()
            };
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
    let pid = get_current_process().unwrap().pid();
    error!("[kernel] {} pid: {}", hint, pid);
    error!("[kernel] instrument at {:#x}", inst_addr);

    exit_current(128);
}

fn restore_to_user() -> ! {
    set_user_trap_entry();
    let satp = PROCESS_MANAGER.get_current_process().unwrap().satp();
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

// TODO: remove this and use a real fs
lazy_static::lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = get_num_app();
        extern "C" { fn _app_names();}
        let mut start = _app_names as *const u8;
        let mut v = vec![];
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != '\0' as u8 {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let s = core::str::from_utf8(slice).unwrap();
                v.push(s);
                start = end.add(1);
            }
        }
        v
    };
    static ref APP_BINS: Vec<&'static [u8]> = {
        let num_apps = get_num_app();
                extern "C" {
            fn _num_app();
        }
        let num_ptr = _num_app as *const usize;
        let app_start = unsafe {
           core::slice::from_raw_parts(num_ptr.add(1), num_apps+1)
        };
        let mut v = vec![];
        for i in 0..num_apps {
            let len = app_start[i+1] - app_start[i];
            if len >= APP_SIZE_LIMIT {
                panic!("app {} is too large", i);
            }
            v.push(
                unsafe {core::slice::from_raw_parts(app_start[i] as *const u8, len)}
            );
        }
        v
    };
}

const MAX_APP_NUM: usize = 16;
const APP_SIZE_LIMIT: usize = 0x40000;

fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    let num_ptr = _num_app as *const usize;
    unsafe { num_ptr.read_volatile() }
}

pub fn app_by_name(path: &str) -> Option<&[u8]> {
    APP_NAMES
        .iter()
        .zip(APP_BINS.iter())
        .find(|(name, _)| path == **name)
        .map(|(_, bin)| *bin)
}
