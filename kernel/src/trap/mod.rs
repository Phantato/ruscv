use core::arch::global_asm;

use riscv::register::{
    scause::{self, Exception, Trap},
    sstatus::{self, Sstatus},
    stval, stvec,
    utvec::TrapMode,
};
global_asm!(include_str!("trap.s"));

use crate::{
    app::{self, get_current_app, restore_to_app},
    error,
    memory::TRAMPOLINE,
    println,
    syscall::syscall,
    trace,
};

pub fn init() {
    set_user_trap_entry();
}

#[repr(C)]
pub struct TrapCtx {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
    pub kernel_satp: usize,
    pub kernel_sp: usize,
    pub trap_handler: usize,
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

#[no_mangle]
fn trap_from_kernel() -> ! {
    panic!("a trap from kernel!");
}

#[no_mangle]
fn trap_from_user() -> ! {
    trace!("trap in");
    set_kernel_trap_entry();
    let app = get_current_app().unwrap();
    let ctx = app.trap_ctx();
    match scause::read().cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.sepc += 4;
            let res = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]]);
            match res {
                Ok(len) => ctx.x[10] = len as usize,
                Err(hint) => kernel_fail(ctx, unsafe { core::str::from_utf8_unchecked(&hint) }),
            }
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            kernel_fail(ctx, "PageFault in application, kernel killed it.");
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            kernel_fail(ctx, "IllegalInstruction in application, kernel killed it.");
        }
        x @ _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", x, stval::read());
        }
    }
    set_user_trap_entry();

    restore_to_app(&app)
}

fn kernel_fail(ctx: &TrapCtx, hint: &str) {
    error!("[kernel]{}", hint);
    println!("instrument at {:#x}", ctx.sepc);

    app::run_next();
}
