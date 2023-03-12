use core::arch::global_asm;

use riscv::register::{
    scause::{self, Exception, Trap},
    sstatus::{self, Sstatus},
    stval, stvec,
    utvec::TrapMode,
};
global_asm!(include_str!("trap.s"));

use crate::{app_manager, println, syscall::syscall};

pub fn init() {
    extern "C" {
        fn __trap();
    }
    unsafe {
        stvec::write(__trap as usize, TrapMode::Direct);
    }
}

#[repr(C)]
pub struct TrapCtx {
    pub x: [usize; 32],
    pub sstatus: Sstatus,
    pub sepc: usize,
}

impl TrapCtx {
    pub fn new_app(entry: usize, sp: usize) -> Self {
        let mut sstatus = sstatus::read();
        sstatus.set_spp(sstatus::SPP::User);
        let mut x = [0; 32];
        x[2] = sp;
        Self {
            x,
            sstatus,
            sepc: entry,
        }
    }
}

#[no_mangle]
fn trap_handler(ctx: &mut TrapCtx) -> &mut TrapCtx {
    match scause::read().cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            ctx.sepc += 4;
            ctx.x[10] = syscall(ctx.x[17], [ctx.x[10], ctx.x[11], ctx.x[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) | Trap::Exception(Exception::StorePageFault) => {
            println!("[kernel] PageFault in application, kernel killed it.");
            app_manager::run_next();
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            println!("[kernel] IllegalInstruction in application, kernel killed it.");
            app_manager::run_next();
        }
        x @ _ => {
            panic!("Unsupported trap {:?}, stval = {:#x}!", x, stval::read());
        }
    }
    ctx
}
