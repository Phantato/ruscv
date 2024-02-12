#[derive(Copy, Clone, PartialEq)]
pub enum ProcessStatus {
    UnInit,  // 未初始化
    Ready,   // 准备运行
    Running, // 正在运行
    Exited,  // 已退出
}
