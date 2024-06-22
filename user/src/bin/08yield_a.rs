#![no_std]
#![no_main]

use user_lib::yield_;

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
    println!("a befor yield");
    yield_();
    println!("a after yield");
    0
}
