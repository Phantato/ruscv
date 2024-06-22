#![no_std]
#![no_main]

use user_lib::{get_time, syscall::sys_yield};

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    let current_timer = get_time();
    let wait_for = current_timer + 3000;
    println!("Before sleep.");
    while get_time() < wait_for {
        sys_yield();
    }
    println!("Test sleep OK!");
    0
}
