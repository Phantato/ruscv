const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

use self::fs::*;
use self::process::*;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}

mod fs {
    use crate::print;

    const FD_STDOUT: usize = 1;

    /// write buf of length `len`  to a file with `fd`
    pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
        match fd {
            FD_STDOUT => {
                let slice = unsafe { core::slice::from_raw_parts(buf, len) };
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
                len as isize
            }
            _ => {
                panic!("Unsupported fd in sys_write!");
            }
        }
    }
}

mod process {
    use crate::{app_manager, println};

    /// task exits and submit an exit code
    pub fn sys_exit(exit_code: i32) -> ! {
        println!("[kernel] Application exited with code {}", exit_code);
        app_manager::run_next()
    }
}
