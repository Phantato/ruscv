use core::arch::asm;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;

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

pub fn sys_exit(xstate: i32) -> isize {
    sys_call(SYSCALL_EXIT, [xstate as usize, 0, 0])
}
