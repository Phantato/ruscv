mod fs;
mod process;

use self::{fs::sys_write, process::*};
use crate::fmt_str;

pub const MAX_MSG_LEN: usize = 32;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;

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
        _ => {
            fmt_str!(error, "Unsupported syscall_id: {:#x}", syscall_id).unwrap();
            Err(())
        }
    }
}
