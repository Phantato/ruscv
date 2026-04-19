use crate::{
    memory::{kernel_stack_position, memory_set::SegmentPermission, VirtAddr, KERNEL_SPACE},
    process::process_control_block::PID,
};

pub struct KernelStack {
    pid: usize,
}

impl KernelStack {
    pub fn new(pid: usize) -> Self {
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        {
            KERNEL_SPACE.get_mut().push_empty_seg(
                kernel_stack_bottom.into(),
                kernel_stack_top.into(),
                SegmentPermission::R | SegmentPermission::W,
            );
        };
        KernelStack { pid }
    }
    pub fn top(&self) -> usize {
        let (_, top) = kernel_stack_position(self.pid);
        top
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);

        let kernel_stack_bottom_va: VirtAddr = kernel_stack_bottom.into();
        KERNEL_SPACE
            .get_mut()
            .remove_area(kernel_stack_bottom_va.into());
    }
}
