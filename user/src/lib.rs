#![no_std]
#![feature(linkage, panic_info_message)]

pub mod console;
pub mod syscall;

use syscall::*;

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    clear_bss();
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

fn clear_bss() {
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) })
}

pub fn write(fd: usize, buf: &[u8]) -> isize {
    sys_write(fd, buf)
}
pub fn exit(exit_code: i32) -> isize {
    sys_exit(exit_code)
}

mod panic {
    use crate::{println, syscall};
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        let unknown_info = format_args!("Unknown Reason");
        let msg = _info.message().unwrap_or(&unknown_info);
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
