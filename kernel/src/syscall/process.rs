use crate::{info, process};

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    process::exit_current()
}

pub fn sys_yield() -> isize {
    process::suspend_current();
    0
}
