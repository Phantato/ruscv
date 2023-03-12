#![no_std]
#![no_main]
#![feature(panic_info_message, set_ptr_value, array_methods, cell_leak)]
mod app_manager;
mod console;
mod sbi;
mod sync;
mod syscall;
mod trap;

use core::arch::global_asm;

use crate::kernel_address::*;

global_asm!(include_str!("entry.s"));
global_asm!(include_str!("link_app.s"));

#[no_mangle]
pub fn rust_main(hartid: usize) -> ! {
    if hartid != 0 {
        unsafe {
            riscv::asm::wfi();
        }
    }
    println!("RUSCV OS Booting on hart: {}", hartid);
    info!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    info!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    info!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    info!(
        "load range : [{:#x}, {:#x}], _start = {:#x}",
        skernel as usize, ekernel as usize, _start as usize
    );

    info!(
        "boot stack alloced at [{:#x}, {:#x}]",
        bstack as usize, tstack as usize
    );

    clear_bss();
    trap::init();
    app_manager::run_next();
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
    }
}

mod panic {
    use crate::println;
    use crate::sbi::shutdown;
    use core::panic::PanicInfo;

    #[panic_handler]
    fn panic(_info: &PanicInfo) -> ! {
        let unknown_info = format_args!("Unknown Reason");
        let msg = _info.message().unwrap_or(&unknown_info);
        if let Some(loc) = _info.location() {
            println!("Kernel Paniced at {}:{} {}!", loc.file(), loc.line(), msg);
        } else {
            println!("{}", msg);
        }
        shutdown()
    }
}
