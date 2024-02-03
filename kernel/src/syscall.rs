const MAX_MSG_LEN: usize = 32;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

// use self::fs::*;
use self::{fs::sys_write, process::*};
use crate::fmt_str;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> Result<isize, [u8; MAX_MSG_LEN]> {
    let mut buf = [0u8; MAX_MSG_LEN];
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1], args[2]).or_else(|msg| {
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

    use alloc::vec::Vec;

    use crate::{
        app::{self, ProcessControlBlock},
        memory::VirtAddr,
        print, println,
    };

    const FD_STDOUT: usize = 1;

    /// write buf of length `len` to a file with `fd`
    pub fn sys_write(fd: usize, buf: usize, len: usize) -> Result<isize, &'static str> {
        if let Some(task) = app::get_current_app() {
            let contents = get_strs(buf, len, &task)?;
            match fd {
                FD_STDOUT => {
                    for ele in contents {
                        print!("{}", ele);
                    }
                    Ok(len as isize)
                }
                _ => Err("Unsupported fd in sys_write!"),
            }
        } else {
            unreachable!("Should have an task running")
        }
    }

    fn get_strs(
        mut buf: usize,
        mut len: usize,
        task: &ProcessControlBlock,
    ) -> Result<Vec<&str>, &'static str> {
        let mut ret = vec![];
        while len > 0 {
            if let Some(pa) = task.translate(buf.into()) {
                let diff = (VirtAddr::from(VirtAddr::from(buf + 1).ceil()).0 - buf).clamp(0, len);
                assert!(diff > 0);
                let slice = unsafe { core::slice::from_raw_parts(pa.0 as *const u8, diff) };
                let s = core::str::from_utf8(slice).unwrap();
                ret.push(s);
                len -= diff;
                buf += diff;
            } else {
                return Err("Address out of range!");
            }
        }
        Ok(ret)
    }
}

mod process {
    use crate::{app, info};

    /// task exits and submit an exit code
    pub fn sys_exit(exit_code: i32) -> ! {
        info!("[kernel] Application exited with code {}", exit_code);
        app::run_next()
    }
}
