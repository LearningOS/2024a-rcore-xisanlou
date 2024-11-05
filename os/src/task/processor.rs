//!Implementation of [`Processor`] and Intersection of control flow
//!
//! Here, the continuous operation of user apps in CPU is maintained,
//! the current running state of CPU is recorded,
//! and the replacement and transfer of control flow of different applications are executed.

// ****** START xisanlou add No.1
use crate::config::MAX_SYSCALL_NUM;
use crate::mm::{VirtAddr, MapPermission};
use crate::timer::get_time_us;
// ****** END xisanlou add No.1
use super::__switch;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;

/// Processor management structure
pub struct Processor {
    ///The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,

    ///The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,
}

impl Processor {
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }

    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }

    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

lazy_static! {
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

///The main part of process execution and scheduling
///Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            // ****** START xisanlou add No.2
            if task_inner.start_time == 0 {
                let us = get_time_us();
                let sec = us / 1_000_000;
                let usec = us % 1_000_000;
                task_inner.start_time = ((sec & 0xffff) * 1000 + usec / 1000) as usize;
            }
            // ****** END xisanlou add No.2
            // release coming task_inner manually
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        } else {
            warn!("no tasks available in run_tasks");
        }
    }
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get the current user token(addr of page table)
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    task.get_user_token()
}

///Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

///Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}

// ****** START xisanlou add No.3
/// Get current task syscall syscall_times
pub fn get_current_task_syscall_times() -> [u32; MAX_SYSCALL_NUM] {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_syscall_times()
}

/// Get current task start time
pub fn get_current_task_start_time() -> usize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_start_time()
}

/// Test virtual address overlapping
pub fn current_user_vpn_no_overlap(start_va: VirtAddr, end_va: VirtAddr) -> bool {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .vpn_no_overlap(start_va, end_va)
}

/// Update current task syscall syscall_times
pub fn update_current_task_syscall_times(update_syscall_times: &usize) {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .update_syscall_times(update_syscall_times);
}

/// insert framed area to user space.
pub fn current_user_insert_framed_area(start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .insert_framed_area(start_va, end_va, permission);
}

/// Unmap current user mapped area in the MemorySet
pub fn current_user_unmap_user_area(start_va: VirtAddr, end_va: VirtAddr) -> isize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .unmap_user_area(start_va, end_va)
}

// ****** xisanlou add at ch5
/// Set pass value for current user
pub fn current_user_set_pass(pass: u64) {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .set_pass(pass);
}

// ****** END xisanlou add No.3

