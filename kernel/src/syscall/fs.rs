use alloc::vec::Vec;

use crate::{
    memory::VirtAddr,
    print,
    process::{get_current_process, ProcessControlBlock},
};

const FD_STDOUT: usize = 1;

/// write buf of length `len` to a file with `fd`
pub fn sys_write(fd: usize, buf: usize, len: usize) -> Result<isize, &'static str> {
    let task = get_current_process();
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
}

fn get_strs(
    mut buf: usize,
    mut len: usize,
    task: &ProcessControlBlock,
) -> Result<Vec<&str>, &'static str> {
    let mut ret = vec![];
    while len > 0 {
        // TODO: this is not safe, because we haven't check the permission.
        if let Some(pa) = task.translate(buf.into()) {
            let diff = (VirtAddr::from(VirtAddr::from(buf + 1).ceil()).0 - buf).clamp(0, len);
            assert!(diff > 0);
            // linearly map physical space rather identical map.
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
