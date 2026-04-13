use core::arch::asm;

const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_FORK: usize = 220;
const SYSCALL_EXEC: usize = 221;
const SYSCALL_WAITPID: usize = 260;


#[inline(always)]
fn syscall(op: usize, args: [usize; 3]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!("ecall",
           inlateout("x10") args[0] => ret,
           in("x11") args[1],
           in("x12") args[2],
           in("x17") op
        )
    }
    ret
}

pub fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    syscall(SYSCALL_READ, [fd, buf.as_mut_ptr() as usize, buf.len()])
}

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    syscall(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(xstate: i32) -> ! {
    syscall(SYSCALL_EXIT, [xstate as usize, 0, 0]);
    unreachable!("program should exited!")
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_YIELD, [0, 0, 0])
}

#[repr(C)]
#[derive(Default)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_get_time() -> isize {
    let mut ts = TimeVal::default();
    let status = syscall(SYSCALL_GET_TIME, [&mut ts as *mut TimeVal as usize, 0, 0]);
    if status != 0 {
        status
    } else {
        (ts.sec * 1000 + ts.usec / 1000) as isize
    }
}

pub fn sys_fork() -> isize{0}
pub fn sys_exec(path:&str) -> isize{
    syscall(SYSCALL_EXEC, [path.as_ptr() as usize, 0, 0])
}
pub fn sys_waitpid(pid: isize, exit_code: *mut i32) -> isize{0}

