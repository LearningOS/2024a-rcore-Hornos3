//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

use crate::timer::get_time_ms;
pub use context::TaskContext;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// Inner of Task Manager
pub struct TaskManagerInner {
    /// task list
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        TaskManager {
            num_app: get_num_app(),
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks: [TaskControlBlock {
                        task_cx: TaskContext::zero_init(),
                        task_status: TaskStatus::UnInit,
                        syscall_counter: [0; MAX_SYSCALL_NUM],
                        start_time: usize::MAX
                    }; MAX_APP_NUM],
                    current_task: 0,
                })
            },
        }
    };
}

/// This constant is used for getting something more conveniently
pub const GET_FOR_CURRENT_TASK: usize = usize::MAX;

impl TaskManager {
    /// Post initialization of TASK_MANAGER
    fn post_initialization(&self) {
        let mut inner = self.inner.exclusive_access();
        for i in 0..MAX_APP_NUM {
            inner.tasks[i].task_cx = TaskContext::goto_restore(init_app_cx(i));
            inner.tasks[i].task_status = TaskStatus::Ready;
        }
    }

    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        task0.start_time = get_time_ms();
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Increase the syscall counter by 1
    fn increase_syscall_counter(&self, syscall_id: usize) {
        let current: usize = self.inner.exclusive_access().current_task;
        let mut inner = self.inner.exclusive_access();
        inner.tasks[current].syscall_counter[syscall_id] += 1;
    }

    /// Get the copy of syscall counter, if received usize::MAX, return the counter of current task
    fn get_syscall_counter(&self, task: usize) -> Result<[u32; MAX_SYSCALL_NUM], &str> {
        match task {
            usize::MAX => {
                let current: usize = self.inner.exclusive_access().current_task;
                let current_task = self.inner.exclusive_access().tasks[current];
                Ok(current_task.syscall_counter.clone())
            }
            x if x < MAX_APP_NUM =>
                Ok(self.inner.exclusive_access().tasks[x].syscall_counter.clone()),
            _ => Err("Invalid task id")
        }
    }

    // Get the last start time of a task, if received usize::MAX, return the time of current task
    fn get_start_time(&self, task: usize) -> Option<usize> {
        let target: usize = if task == GET_FOR_CURRENT_TASK {
            self.inner.exclusive_access().current_task
        } else {
            task
        };
        let current_task = self.inner.exclusive_access().tasks[target];
        let now = get_time_ms();
        match current_task.start_time {
            usize::MAX => None,
            x if x <= now => Some(now - x),
            _ => panic!("Corrupted start timestamp")
        }
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            if inner.tasks[next].start_time == usize::MAX {
                inner.tasks[next].start_time = get_time_ms();
            }
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }
}

/// Post initialization
pub fn post_initialization() {
    TASK_MANAGER.post_initialization();
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Increase the syscall usage counter of current task
#[allow(unused)]
pub fn increase_syscall_counter(syscall_id: usize) {
    TASK_MANAGER.increase_syscall_counter(syscall_id);
}

/// Get the syscall usage counter of a task (MAX_SYSCALL_NUM for current task)
#[allow(unused)]
pub fn get_syscall_counter(task: usize) -> Result<[u32; MAX_SYSCALL_NUM], &'static str> {
    TASK_MANAGER.get_syscall_counter(task)
}

/// Get the last start time of a task (MAX_SYSCALL_NUM for current task)
#[allow(unused)]
pub fn get_start_time(task: usize) -> Option<usize> {
    TASK_MANAGER.get_start_time(task)
}