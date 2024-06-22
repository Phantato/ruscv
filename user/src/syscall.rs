use core::arch::asm;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;

#[inline(always)]
fn sys_call(op: usize, args: [usize; 3]) -> isize {
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

pub fn sys_write(fd: usize, buffer: &[u8]) -> isize {
    sys_call(SYSCALL_WRITE, [fd, buffer.as_ptr() as usize, buffer.len()])
}

pub fn sys_exit(xstate: i32) -> ! {
    sys_call(SYSCALL_EXIT, [xstate as usize, 0, 0]);
    unreachable!("program should exited!")
}

pub fn sys_yield() -> isize {
    sys_call(SYSCALL_YIELD, [0, 0, 0])
}

#[repr(C)]
#[derive(Default)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

pub fn sys_get_time() -> isize {
    let mut ts = TimeVal::default();
    let status = sys_call(SYSCALL_GET_TIME, [&mut ts as *mut TimeVal as usize, 0, 0]);
    if status != 0 {
        status
    } else {
        (ts.sec * 1000 + ts.usec / 1000) as isize
    }
}
