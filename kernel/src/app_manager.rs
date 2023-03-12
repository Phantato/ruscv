#![allow(unused)]
use core::{
    arch::asm,
    array, fmt, hint,
    mem::{self, MaybeUninit},
};

use crate::{println, sbi, sync::UPSafeCell, trace, trap::TrapCtx};

const USER_STACK_SIZE: usize = 4096 * 2;
const KERNEL_STACK_SIZE: usize = 4096 * 2;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x40000;

lazy_static::lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
        UPSafeCell::new(AppManager::new())
    };
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};

static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

struct KernelStack {
    data: [usize; KERNEL_STACK_SIZE],
}
struct UserStack {
    data: [usize; USER_STACK_SIZE],
}

impl KernelStack {
    fn get_bottom(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }
    fn new_context(&self) -> *const TrapCtx {
        let ctx_ptr = (self.get_bottom() - core::mem::size_of::<TrapCtx>()) as *mut TrapCtx;
        unsafe {
            *ctx_ptr = TrapCtx::new_app(APP_BASE_ADDRESS, USER_STACK.get_bottom());
        }
        ctx_ptr
    }
}

impl UserStack {
    fn get_bottom(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

struct AppManager {
    num: usize,
    current: usize,
    interval: [usize; MAX_APP_NUM + 1],
}

impl fmt::Display for AppManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe {
            let mut debug = f.debug_struct("AppManager");
            debug
                .field("num", &self.num)
                .field("current", &self.current);

            for i in 0..self.num {
                let mut field_name = *b"app_00";
                field_name[4] += i as u8 / 10;
                field_name[5] += i as u8 % 10;
                debug.field(
                    core::str::from_utf8_unchecked(&field_name),
                    &((self.interval)[i]..(self.interval)[i + 1]),
                );
            }
            debug.finish()
        }
    }
}

impl AppManager {
    pub unsafe fn new() -> Self {
        extern "C" {
            fn _num_app();
        }
        let num_app_ptr = _num_app as *const usize;
        let num = num_app_ptr.read_volatile();
        let interval_raw = core::slice::from_raw_parts(num_app_ptr.add(1), num + 1);
        let mut interval = [0; 17];
        interval[..=num].copy_from_slice(interval_raw);
        Self {
            num,
            interval,
            current: 0,
        }
    }

    unsafe fn load(&self, id: usize) {
        if id >= self.num {
            println!("no more app to execute");
            sbi::shutdown()
        }
        trace!("[kernel] Loading app_{}", id);
        // clear app area
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);
        let app_src = core::slice::from_raw_parts(
            self.interval[id] as *const u8,
            self.interval[id + 1] - self.interval[id],
        );
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
        // memory fence about fetching the instruction memory
        asm!("fence.i");
    }
}

pub fn run_next() -> ! {
    {
        let mut app_manager = APP_MANAGER.borrow_mut();
        unsafe {
            app_manager.load(app_manager.current);
        }
        app_manager.current += 1;
    }

    extern "C" {
        fn __restore(cx_addr: usize);
    }
    unsafe {
        __restore(KERNEL_STACK.new_context() as usize);
    }
    unreachable!()
}
