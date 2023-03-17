#![allow(unused)]
use core::{
    arch::asm,
    array,
    borrow::Borrow,
    default, ffi, fmt, hint,
    mem::{self, MaybeUninit},
    ops::AddAssign,
};

use crate::{print, println, sbi, sync::UPSafeCell, trace, trap::TrapCtx};

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
#[derive(Default, Clone, Copy)]
struct App {
    start: usize,
    len: usize,
    seq_no: usize,
    name: usize,
}

impl fmt::Display for App {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\t[prog no.{}]:{:?}      \tstart from:{:#x},\tlen:{:#x}",
            self.seq_no,
            unsafe { ffi::CStr::from_ptr(self.name as *const _) },
            self.start,
            self.len
        )
    }
}

struct AppManager {
    num: usize,
    next: usize,
    load: [App; MAX_APP_NUM + 1],
}

impl fmt::Display for AppManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "num: {}\r", self.num)?;
        writeln!(f, "current: {}\r", self.next - 1)?;
        writeln!(f, "all loads: [\r")?;
        for i in 0..self.num {
            writeln!(f, "{}\r", self.load[i])?
        }
        writeln!(f, "]\r")
    }
}

impl AppManager {
    pub unsafe fn new() -> Self {
        extern "C" {
            fn _num_app();
        }
        let num_app_ptr = _num_app as *const usize;
        let num = num_app_ptr.read_volatile();
        println!("{}", num);
        let mut load: [App; 17] = Default::default();
        let mut interval_cursor = num_app_ptr.add(1);
        let mut name_cursor = interval_cursor.add(num + 1) as *const u8;

        for i in 0..num {
            load[i].seq_no = i;
            load[i].start = *interval_cursor;
            interval_cursor = interval_cursor.add(1);
            load[i].len = *interval_cursor - load[i].start;
            load[i].name = name_cursor as usize;
            while *name_cursor != 0 {
                name_cursor = name_cursor.add(1);
            }
            name_cursor = name_cursor.add(1);
        }

        Self { num, load, next: 0 }
    }

    unsafe fn load(&self, id: usize) {
        if id >= self.num {
            println!("no more app to execute");
            sbi::shutdown()
        }
        let app = &self.load[id];
        trace!("[kernel] Loading app_{}", id);
        // clear app area
        core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, APP_SIZE_LIMIT).fill(0);

        let app_src = core::slice::from_raw_parts(app.start as *const u8, app.len);
        let app_dst = core::slice::from_raw_parts_mut(APP_BASE_ADDRESS as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
        // memory fence about fetching the instruction memory
        asm!("fence.i");
    }
}

pub fn run_next() -> ! {
    {
        let mut app_manager = APP_MANAGER.get_mut();
        unsafe {
            app_manager.load(app_manager.next);
        }
        app_manager.next += 1;
    }

    extern "C" {
        fn __restore(cx_addr: usize);
    }
    unsafe {
        __restore(KERNEL_STACK.new_context() as usize);
    }
    unreachable!()
}

pub fn print_loads() {
    let inner = APP_MANAGER.get();
    println!("Loaded Apps: {}", inner);
}

pub fn addr_valid(addr: usize) -> bool {
    let inner = APP_MANAGER.get();
    let current_app = inner.load[inner.next - 1];
    drop(inner);
    let current_app_start = APP_BASE_ADDRESS;
    let current_app_end = APP_BASE_ADDRESS + current_app.len;
    if addr >= current_app_start && addr < current_app_end {
        return true;
    }
    let stack_top: usize;
    unsafe {
        asm!("mv {}, sp", out(reg) stack_top);
    }
    if addr > stack_top && addr <= USER_STACK.get_bottom() {
        return true;
    }

    return false;
}

pub fn current_instrument_location(addr: usize) -> usize {
    addr
    // let offset = addr - APP_BASE_ADDRESS;
    // let inner = APP_MANAGER.get();
    // let current_start = inner.load[inner.next - 1].start;
    // println!("addr: {:#x}, start: {:#x}", addr, current_start);

    // return current_start + offset;
}
