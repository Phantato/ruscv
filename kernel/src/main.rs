#![no_std]
#![no_main]
#![feature(
    panic_info_message,
    set_ptr_value,
    array_methods,
    cell_leak,
    ptr_from_ref,
    step_trait,
    alloc_error_handler
)]
#[macro_use]
extern crate alloc;

mod configs;
mod console;
mod kernel_heap;
mod memory;
mod process;
mod sbi;
mod sync;
mod syscall;
mod utils;

use crate::kernel_address::*;
use core::arch::global_asm;

global_asm!(include_str!("entry.s"));
global_asm!(include_str!("link_app.s"));

#[no_mangle]
pub fn rust_main(hartid: usize) -> ! {
    if hartid != 0 {
        unsafe {
            riscv::asm::wfi();
        }
    }
    info!("RUSCV OS Booting on hart: {}", hartid);
    info!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    info!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    info!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    info!(".bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
    info!(
        "kernel image : [{:#x}, {:#x}], _start = {:#x}",
        skernel as usize, ekernel as usize, _start as usize
    );
    info!(
        "boot stack alloced at [{:#x}, {:#x}]",
        bstack as usize, tstack as usize
    );

    clear_bss();

    kernel_heap::init();
    kernel_heap::test();

    memory::init();
    memory::test();

    process::run_next_process();
}

fn clear_bss() {
    (sbss as usize..ebss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}

mod kernel_address {

    #[allow(unused)]
    extern "C" {
        pub fn _start();

        pub fn skernel();
        pub fn ekernel();
        pub fn stext();
        pub fn etext();
        pub fn srodata();
        pub fn erodata();
        pub fn sdata();
        pub fn edata();
        pub fn sbss();
        pub fn ebss();
        pub fn bstack();
        pub fn tstack();

        pub fn strampoline();
    }
}

mod panic {
    use crate::{println, sbi::shutdown};
    use core::{arch::asm, panic::PanicInfo, ptr};

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        let unknown_info = format_args!("Unknown Reason");
        let msg = _info.message().unwrap_or(&unknown_info);
        if let Some(loc) = _info.location() {
            println!("Kernel Paniced at {}:{} {}!", loc.file(), loc.line(), msg);
        } else {
            println!("{}", msg);
        }
        print_stack_trace();
        shutdown(true)
    }

    pub fn print_stack_trace() -> () {
        println!("== Begin stack trace ==");

        let mut fp: *const usize;
        unsafe {
            asm!("mv {}, fp", out(reg) fp);
            while fp != ptr::null() {
                let saved_ra = *fp.sub(1);
                let saved_fp = *fp.sub(2);

                println!("ra = 0x{:016x}, fp = 0x{:016x}", saved_ra, saved_fp);

                fp = saved_fp as *const usize;
            }
        }
        println!("== End stack trace ==");
    }

    #[alloc_error_handler]
    pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
        panic!("Heap allocation error, layout = {:?}", layout);
    }
}
