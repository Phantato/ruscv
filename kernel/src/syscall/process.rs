use riscv::addr::PhysAddr;

use crate::{
    info,
    memory::{PTEFlags, VirtAddr},
    process::{self, ProcessStatus},
};

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    process::exit_current(exit_code)
}

pub fn sys_yield() -> Result<isize, ()> {
    process::suspend_current();
    Ok(0)
}

pub fn sys_fork() -> Result<isize, ()> {
    let child = process::fork_current();
    Ok(child.map_or(-1, |pcb| pcb.pid().to_raw()))
}

pub fn sys_exec(path: *const u8) -> Result<isize, ()> {
    let path = {
        let current = process::get_current_process().unwrap();
        let inner = current.inner.get();
        inner.get_user_str(path)?
    };
    if let Some(data) = process::app_by_name(&path) {
        process::exec(data)?;
        Ok(0)
    } else {
        Ok(-1)
    }
}

pub fn sys_waitpid(pid: isize, exit_code_ptr: usize) -> Result<isize, ()> {
    let parent = process::get_current_process().unwrap();
    let mut inner = parent.inner.get_mut();
    if inner
        .children
        .iter()
        .find(|child| pid == -1 || child.pid().to_raw() == pid)
        .is_none()
    {
        return Ok(-1);
    }

    let Some((index, _)) = inner
        .children
        .iter()
        .enumerate()
        .filter(|(_, child)| pid == -1 || child.pid().to_raw() == pid)
        .find(|(_, child)| child.inner.get().status == ProcessStatus::Exited)
    else {
        return Ok(-2);
    };
    let child = inner.children.swap_remove(index);
    let exit_addr = inner.translate(VirtAddr(exit_code_ptr), PTEFlags::W)?;
    unsafe {
        *exit_addr.get_mut().ok_or(())? = child.inner.get().exit_code;
    }

    Ok(child.pid().to_raw())
}
