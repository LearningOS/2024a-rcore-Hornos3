//! Types related to task management

use crate::config::MAX_SYSCALL_NUM;
use super::TaskContext;

/// The syscall counter, use bitmask to shrink its size
#[derive(Copy, Clone)]
#[allow(unused)]
pub struct SyscallCtr {
    /// The bitmask, if a syscall is called, the corresponding bit is set to 1, otherwise 0
    pub bitmask: [u8; MAX_SYSCALL_NUM / 8 + 1],
    /// counter index, if the syscall 100 is saved in index 0, then counter_idx[0] = 100
    pub counter_idx: [u16; 1],
    /// counter
    pub counter: [u32; 1]
}

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in its lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// Syscall counters
    pub syscall_counter: [u32; MAX_SYSCALL_NUM],
    /// start timestamp (ms)
    pub start_time: usize
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
