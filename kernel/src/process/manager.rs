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
    current: Option<Arc<ProcessControlBlock>>,
    ready: VecDeque<Arc<ProcessControlBlock>>,
    load: BTreeMap<PID, Arc<ProcessControlBlock>>,
}

const CURRENT_PROCESS_NOT_EXIST_ERR: &str = "current pcb should exist";

impl ProcessManager {
    unsafe fn new() -> Self {
        let load = super::APP_BINS
            .iter()
            .map(|elf| {
                let pcb = ProcessControlBlock::from_elf(*elf).unwrap();
                (pcb.pid(), pcb)
            })
            .collect::<BTreeMap<_, _>>();
        trace!("bin num: {}", load.len());

        let inner = UPSafeCell::new(ProcessManagerInner {
            current: None,
            ready: load.values().map(|pcb| pcb.clone()).collect(),
            load,
        });
        Self { inner }
    }

    pub(super) fn start(&self) -> ! {
        self.run_next_process();
        unreachable!("unreachable in ProcessManager::start")
    }

    pub fn run_next_process(&self) {
        let current_ctx = self
            .get_current_switch_ctx()
            .unwrap_or(0 as *mut SwitchCtx);
        let next_ctx = {
            let mut inner = self.inner.get_mut();
            let Some(pcb) = inner.ready.pop_front() else {
                shutdown(false);
            };
            inner.current = Some(pcb.clone());
            let mut inner = pcb.inner.get_mut();
            inner.status = ProcessStatus::Running;
            &inner.switch_ctx as *const SwitchCtx
        };
        unsafe { __switch(current_ctx, next_ctx) }
    }

    pub(super) fn mark_current_ready(&self) {
        let inner = self.inner.get_mut();
        let pcb = inner.must_get_current_pcb();
        self.mark_ready(pcb);
    }

    fn mark_ready(&self, pcb: Arc<ProcessControlBlock>) {
        let mut inner = self.inner.get_mut();
        {
            let mut pcb_inner = pcb.inner.get_mut();
            pcb_inner.status = ProcessStatus::Ready;
        }
        inner.ready.push_back(pcb);
    }

    pub(super) fn mark_current_exited(&self, exit_code: i32) {
        let mut inner = self.inner.get_mut();
        let Some(pcb) = inner.current.clone() else {
            shutdown(true);
        };
        inner.remove(&pcb.pid());
        let mut inner = pcb.inner.get_mut();
        inner.exit_code = exit_code;
        inner.status = ProcessStatus::Exited;
    }

    pub(super) fn fork_current(&self) -> Result<Arc<ProcessControlBlock>, ()> {
        let mut inner = self.inner.get_mut();
        let pcb = inner.current.clone().ok_or(())?;
        pcb.fork().inspect(|child| {
            let mut parent = pcb.inner.get_mut();
            parent.children.push(child.clone());
            inner.ready.push_back(child.clone());
            inner.append(child.clone());
        })
    }
    pub(super) fn exec(&self, elf: &[u8]) -> Result<(), ()> {
        let current = self.get_current_process().ok_or(())?;
        current.exec(elf)
    }

    pub(super) fn get_current_process(&self) -> Option<Arc<ProcessControlBlock>> {
        self.inner.get().current.clone()
    }

    pub fn get_current_switch_ctx(&self) -> Option<*mut SwitchCtx> {
        let pcb = self.inner.get().current.clone()?;
        let mut inner = pcb.inner.get_mut();
        Some(&mut inner.switch_ctx as *mut SwitchCtx)
    }
}

impl ProcessManagerInner {
    fn get_current_pcb(&self) -> Option<Arc<ProcessControlBlock>> {
        self.current.clone()
    }
    fn must_get_current_pcb(&self) -> Arc<ProcessControlBlock> {
        self.get_current_pcb().expect(CURRENT_PROCESS_NOT_EXIST_ERR)
    }
    fn append(&mut self, pcb: Arc<ProcessControlBlock>) {
        self.load.insert(pcb.pid(), pcb);
    }
    fn remove(&mut self, pid: &PID) {
        self.load.remove(pid);
    }
}
