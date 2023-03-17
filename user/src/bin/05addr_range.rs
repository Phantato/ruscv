#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use core::arch::asm;

use user_lib::syscall::sys_write;
const STDOUT: usize = 1;

#[no_mangle]
fn main() -> i32 {
    println!("Try to access stack address");
    println!("This should be fine");
    let sp: usize;
    unsafe { asm!("mv {}, sp", out(reg) sp) };
    let mut buf = [0u8; 29];
    buf.copy_from_slice(b"Hello world from 0000000000\r\n");
    buf[26] += (sp % 10) as u8;
    sys_write(STDOUT, &buf);
    println!("Try to access address out of app range");
    println!("Kernel should kill this application!");
    sys_write(STDOUT, unsafe {
        core::slice::from_raw_parts(0x8000_0000 as *const _, 10)
    });
    0
}
