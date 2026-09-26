mod constant;
mod fs;
mod process;

use self::{fs::*, process::*};
use crate::{
    fmt_str,
    memory::PTEFlags,
    process::get_current_process,
    timer::{get_time_us, MICRO_PER_SEC},
};
pub use constant::*;

pub const MAX_MSG_LEN: usize = 32;
// use self::fs::*;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(
    syscall_id: usize,
    args: [usize; 3],
    error: &mut [u8; MAX_MSG_LEN],
) -> Result<isize, ()> {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1], args[2]).or_else(|msg| {
            fmt_str!(error, "{}", msg).unwrap();
            Err(())
        }),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0], args[1]),
        SYSCALL_FORK => sys_fork(),
        SYSCALL_EXEC => sys_exec(args[0] as *const u8),
        SYSCALL_WAITPID => sys_waitpid(args[0] as isize, args[1]),
        _ => {
            fmt_str!(error, "Unsupported syscall_id: {:#x}", syscall_id).unwrap();
            Err(())
        }
    }
}

fn sys_get_time(va: usize, _tz: usize) -> Result<isize, ()> {
    let pcb = get_current_process().unwrap();
    let inner = pcb.inner.get();
    let ts = inner.translate(va.into(), PTEFlags::W)?;
    let t = get_time_us();
    unsafe {
        *ts.get_mut().ok_or(())? = TimeVal {
            sec: t / MICRO_PER_SEC,
            usec: t % MICRO_PER_SEC,
        };
    }
    Ok(0)
}

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}
