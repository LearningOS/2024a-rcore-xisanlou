//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
//use alloc::collections::VecDeque;
// ****** START xisanlou add No.1
use alloc::collections::binary_heap::BinaryHeap;
use core::cmp::Reverse;
// ****** END xisanlou add No.1
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    //ready_queue: VecDeque<Arc<TaskControlBlock>>,

    // ****** START xisanlou add No.2
    ready_queue_0: BinaryHeap<Reverse<Arc<TaskControlBlock>>>,
    ready_queue_1: BinaryHeap<Reverse<Arc<TaskControlBlock>>>,
    push_0: bool,
    pop_0: bool,
    // ****** END xisanlou add No.2
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            //ready_queue: VecDeque::new(),

            // ****** START xisanlou add No.3
            ready_queue_0: BinaryHeap::new(),
            ready_queue_1: BinaryHeap::new(),
            push_0: true,
            pop_0: true,
            // ****** END xisanlou add No.3
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        //self.ready_queue.push_back(task);

        // ****** START xisanlou add No.4
        // switch push queue
        if task.stride_overflow() && self.push_0 == self.pop_0 {
            self.push_0 = !self.push_0;
        }

        task.stride_add_step();

        if self.push_0 {
            self.ready_queue_0.push(Reverse(task));
        } else {
            self.ready_queue_1.push(Reverse(task));
        }
        // ****** END xisanlou add No.4
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        //self.ready_queue.pop_front()

        // ****** START xisanlou add No.5
        let  option_r_task: Option<Reverse<Arc<TaskControlBlock>>>;
        let  option_r_task_2: Option<Reverse<Arc<TaskControlBlock>>>;
        if self.pop_0 {
            option_r_task = self.ready_queue_0.pop();
        } else {
            option_r_task = self.ready_queue_1.pop();
        }

        match option_r_task {
            Some(r_task) => {return Some(r_task.0);},
            None => {
                // Switch pop queue
                if self.pop_0 != self.push_0 {
                    self.pop_0 = !self.pop_0;
                    if self.pop_0 {
                        option_r_task_2 = self.ready_queue_0.pop();
                    } else {
                        option_r_task_2 = self.ready_queue_1.pop();
                    }
                    match option_r_task_2 {
                        Some(r_task) => {return Some(r_task.0);},
                        None => {return None;},
                    }
                } else {
                    return None;
                }
            },
        }
        // ****** END xisanlou add No.5
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
