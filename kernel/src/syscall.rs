const MAX_MSG_LEN: usize = 32;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

use self::fs::*;
use self::process::*;
use crate::fmt_str;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> Result<isize, [u8; MAX_MSG_LEN]> {
    let mut buf = [0u8; MAX_MSG_LEN];

    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]).or_else(|msg| {
            fmt_str!(&mut buf, "{}", msg).unwrap();
            Err(buf)
        }),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        _ => {
            fmt_str!(&mut buf, "Unsupported syscall_id: {:#x}", syscall_id).unwrap();
            Err(buf)
        }
    }
}

mod fs {
    use crate::{app, print};

    const FD_STDOUT: usize = 1;

    /// write buf of length `len`  to a file with `fd`
    pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> Result<isize, &'static str> {
        let buf_end = unsafe { buf.add(len - 1) };
        if app::addr_valid(buf as usize) && app::addr_valid(buf_end as usize) {
            match fd {
                FD_STDOUT => {
                    let slice = unsafe { core::slice::from_raw_parts(buf, len) };
                    let str = core::str::from_utf8(slice).unwrap();
                    print!("{}", str);
                    Ok(len as isize)
                }
                _ => Err("Unsupported fd in sys_write!"),
            }
        } else {
            Err("Address out of range!")
        }
    }
}

mod process {
    use crate::{app, println};

    /// task exits and submit an exit code
    pub fn sys_exit(exit_code: i32) -> ! {
        println!("[kernel] Application exited with code {}", exit_code);
        app::run_next()
    }
}
