mod fs;
mod process;

use self::{fs::sys_write, process::*};
use crate::{
    fmt_str,
    process::get_current_process,
    timer::{get_time_us, MICRO_PER_SEC},
};

pub const MAX_MSG_LEN: usize = 32;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
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
        SYSCALL_YIELD => Ok(sys_yield()),
        SYSCALL_GET_TIME => sys_get_time(args[0], args[1]),
        _ => {
            fmt_str!(error, "Unsupported syscall_id: {:#x}", syscall_id).unwrap();
            Err(())
        }
    }
}
fn sys_get_time(va: usize, _tz: usize) -> Result<isize, ()> {
    let task = get_current_process();
    let pa = task.translate(va.into()).ok_or(())?;
    // TODO: this is not safe, because we haven't check the permission.
    let ts = pa.0 as *mut TimeVal;
    let t = get_time_us();
    unsafe {
        (*ts).sec = t / MICRO_PER_SEC;
        (*ts).usec = t % MICRO_PER_SEC;
    }

    Ok(0)
}

#[repr(C)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}
