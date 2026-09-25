use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    vec::Vec,
};

use crate::{
    process::{
        __switch, process_control_block::PID, status::ProcessStatus, ProcessControlBlock, SwitchCtx,
    },
    sbi::shutdown,
    sync::UPSafeCell,
    trace,
};

lazy_static::lazy_static! {
    pub(super) static ref PROCESS_MANAGER: ProcessManager = unsafe {
        ProcessManager::new()
    };
}

pub(super) struct ProcessManager {
    inner: UPSafeCell<ProcessManagerInner>,
}
struct ProcessManagerInner {
    current: Option<PID>,
    ready: VecDeque<PID>,
    load: BTreeMap<PID, Arc<ProcessControlBlock>>,
}

const CURRENT_PROCESS_NOT_EXIST_ERR: &str = "current pcb should exist";

impl ProcessManager {
    unsafe fn new() -> Self {
        let load = APP_BINS
            .iter()
            .map(|elf| {
                let pcb = ProcessControlBlock::from_elf(*elf).unwrap();
                (pcb.pid(), pcb)
            })
            .collect::<BTreeMap<_, _>>();
        trace!("bin num: {}", load.len());

        let inner = UPSafeCell::new(ProcessManagerInner {
            current: None,
            ready: load.keys().map(|pid| *pid).collect(),
            load,
        });
        Self { inner }
    }

    pub(super) fn start(&self) -> ! {
        self.run_next_process(&mut SwitchCtx::zero() as *mut SwitchCtx);
        unreachable!("unreachable in ProcessManager::start")
    }

    pub fn run_next_process(&self, current_ctx: *mut SwitchCtx) {
        match self.inner.get_mut().ready.pop_front() {
            Some(pid) => {
                let next_ctx = {
                    let mut inner = self.inner.get_mut();
                    inner.current = Some(pid);
                    let pcb = inner
                        .load
                        .get(&pid)
                        .expect(&format!("pcb {} should exist", pid.to_raw()));

                    let mut inner = pcb.inner.get_mut();
                    inner.status = ProcessStatus::Running;
                    &inner.switch_ctx as *const SwitchCtx
                };
                unsafe { __switch(current_ctx, next_ctx) }
            }
            None => shutdown(false),
        }
    }

    pub(super) fn mark_current_ready(&self) {
        let mut inner = self.inner.get_mut();
        let current = inner.current.unwrap();
        let pcb = inner.must_get_current_pcb();
        let mut pcb_inner = pcb.inner.get_mut();
        pcb_inner.status = ProcessStatus::Ready;
        inner.ready.push_back(current);
    }
    // FIXME: recycle parent-children relation, or do it while drop pcb
    pub(super) fn mark_current_exited(&self, exit_code: i32) {
        let pcb = self.must_get_current_process();
        let mut inner = pcb.inner.get_mut();
        inner.exit_code = exit_code;
        inner.status = ProcessStatus::Exited;
    }
    pub(super) fn fork_current(&self) -> Result<Arc<ProcessControlBlock>, ()> {
        let pcb = self.get_current_process().ok_or(())?;
        pcb.fork().inspect(|child| {
            let mut inner = self.inner.get_mut();
            let mut parent = pcb.inner.get_mut();
            parent.children.push(child.pid());
            inner.append(child);
        })
    }

    pub fn get_current_process(&self) -> Option<Arc<ProcessControlBlock>> {
        self.inner.get().get_current_pcb()
    }
    pub fn must_get_current_process(&self) -> Arc<ProcessControlBlock> {
        self.inner.get().must_get_current_pcb()
    }
    pub(super) fn get_current_switch_ctx(&self) -> *mut SwitchCtx {
        let pcb = self.must_get_current_process();
        let mut inner = pcb.inner.get_mut();
        &mut inner.switch_ctx as *mut SwitchCtx
    }
    pub(super) fn get_current_satp(&self) -> usize {
        self.must_get_current_process().satp()
    }
}

impl ProcessManagerInner {
    fn get_current_pcb(&self) -> Option<Arc<ProcessControlBlock>> {
        self.current
            .and_then(|pid| self.load.get(&pid))
            .map(|pcb| pcb.clone())
    }
    fn must_get_current_pcb(&self) -> Arc<ProcessControlBlock> {
        self.get_current_pcb().expect(CURRENT_PROCESS_NOT_EXIST_ERR)
    }
    fn append(&mut self, pcb: &Arc<ProcessControlBlock>) {
        self.load.insert(pcb.pid(), pcb.clone());
        self.ready.push_back(pcb.pid());
    }
}

// TODO: remove this and use a real fs
lazy_static::lazy_static! {
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = get_num_app();
        extern "C" { fn _app_names();}
        let mut start = _app_names as *const u8;
        let mut v = vec![];
        unsafe {
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != '\0' as u8 {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                let s = core::str::from_utf8(slice).unwrap();
                v.push(s);
                start = end.add(1);
            }
        }
        v
    };
    static ref APP_BINS: Vec<&'static [u8]> = {
        let num_apps = get_num_app();
                extern "C" {
            fn _num_app();
        }
        let num_ptr = _num_app as *const usize;
        let app_start = unsafe {
           core::slice::from_raw_parts(num_ptr.add(1), num_apps+1)
        };
        let mut v = vec![];
        for i in 0..num_apps {
            let len = app_start[i+1] - app_start[i];
            if len >= APP_SIZE_LIMIT {
                panic!("app {} is too large", i);
            }
            v.push(
                unsafe {core::slice::from_raw_parts(app_start[i] as *const u8, len)}
            );
        }
        v
    };
}

const MAX_APP_NUM: usize = 16;
const APP_SIZE_LIMIT: usize = 0x40000;

fn get_num_app() -> usize {
    extern "C" {
        fn _num_app();
    }
    let num_ptr = _num_app as *const usize;
    unsafe { num_ptr.read_volatile() }
}

fn app_by_name(path: &str) -> Option<&[u8]> {
    APP_NAMES
        .iter()
        .zip(APP_BINS.iter())
        .find(|(name, _)| path == **name)
        .map(|(_, bin)| *bin)
}
