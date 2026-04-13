#![no_std]
#![feature(linkage)]

pub mod console;
pub mod syscall;

mod alloc;

use syscall::*;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    exit(main());
    unreachable!()
}
#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}
extern "C" {
    pub fn sbss();
    pub fn ebss();
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    sys_read(fd, buf)
}

const STDIN: usize = 0;

pub fn getchar() -> u8 {
    let mut c = [0u8; 1];
    read(STDIN, &mut c);
    c[0]
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

pub fn yield_() -> isize {
    sys_yield()
}

pub fn get_time() -> isize {
    sys_get_time()
}

pub fn fork() -> isize {
    sys_fork()
}

pub fn exec(path: &str) -> isize {
    sys_exec(path)
}

pub fn wait(exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(-1, exit_code as *mut _) {
            -2 => {yield_();}
            exit_pid => return exit_pid,
        }
    }
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    loop {
        match sys_waitpid(pid as isize, exit_code as *mut _) {
            -2 => {yield_();}
            exit_pid => return exit_pid,
        }
    }    
}

mod panic {
    use crate::{println, syscall};
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        let unknown_info = "Unknown Reason";
        let msg = _info.message().as_str().unwrap_or(&unknown_info);
        if let Some(loc) = _info.location() {
            println!(
                "\x1b[31m[Panic] at {}:{} {}!\x1b[0m",
                loc.file(),
                loc.line(),
                msg
            );
        } else {
            println!("\x1b[31m[Panic] {}\x1b[0m", msg);
        }
        syscall::sys_exit(-1)
    }
}
