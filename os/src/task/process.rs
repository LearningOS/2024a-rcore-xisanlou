//! Implementation of  [`ProcessControlBlock`]

use super::id::RecycleAllocator;
use super::manager::insert_into_pid2process;
use super::TaskControlBlock;
use super::{add_task, SignalFlags};
use super::{pid_alloc, PidHandle};
use crate::fs::{File, Stdin, Stdout};
use crate::mm::{translated_refmut, MemorySet, KERNEL_SPACE};
use crate::sync::{Condvar, Mutex, Semaphore, UPSafeCell};
use crate::trap::{trap_handler, TrapContext};
use alloc::string::String;
use alloc::sync::{Arc, Weak};
use alloc::vec;
use alloc::vec::Vec;
use core::cell::RefMut;

/// Process Control Block
pub struct ProcessControlBlock {
    /// immutable
    pub pid: PidHandle,
    /// mutable
    inner: UPSafeCell<ProcessControlBlockInner>,
}

/// Inner of Process Control Block
pub struct ProcessControlBlockInner {
    /// is zombie?
    pub is_zombie: bool,
    /// memory set(address space)
    pub memory_set: MemorySet,
    /// parent process
    pub parent: Option<Weak<ProcessControlBlock>>,
    /// children process
    pub children: Vec<Arc<ProcessControlBlock>>,
    /// exit code
    pub exit_code: i32,
    /// file descriptor table
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    /// signal flags
    pub signals: SignalFlags,
    /// tasks(also known as threads)
    pub tasks: Vec<Option<Arc<TaskControlBlock>>>,
    /// task resource allocator
    pub task_res_allocator: RecycleAllocator,
    /// mutex list
    pub mutex_list: Vec<Option<Arc<dyn Mutex>>>,
    /// semaphore list
    pub semaphore_list: Vec<Option<Arc<Semaphore>>>,
    /// condvar list
    pub condvar_list: Vec<Option<Arc<Condvar>>>,
    // ****** START xisanlou add at ch8 No.1
    pub enable_deadlock_detect: bool,
    pub mutex_allocation: Vec<Vec<usize>>,
    pub semaphore_available: Vec<isize>,
    pub semaphore_allocation: Vec<Vec<isize>>,
    pub semaphore_need: Vec<Vec<isize>>,
    pub finish: Vec<bool>,
    pub semaphore_wait_queue: Vec<Option<Arc<TaskControlBlock>>>,
    // ****** END xisanlou add at ch8 No.1
}

impl ProcessControlBlockInner {
    #[allow(unused)]
    /// get the address of app's page table
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// allocate a new file descriptor
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
    /// allocate a new task id
    pub fn alloc_tid(&mut self) -> usize {
        self.task_res_allocator.alloc()
    }
    /// deallocate a task id
    pub fn dealloc_tid(&mut self, tid: usize) {
        self.task_res_allocator.dealloc(tid)
    }
    /// the count of tasks(threads) in this process
    pub fn thread_count(&self) -> usize {
        self.tasks.len()
    }
    /// get a task with tid in this process
    pub fn get_task(&self, tid: usize) -> Arc<TaskControlBlock> {
        self.tasks[tid].as_ref().unwrap().clone()
    }

    // ****** START xisanlou add at ch8 No.11
    /// test task have enough resource to running
    pub fn task_running_has_enough_semaphores(&self, tid: usize) -> bool {
        if self.finish[tid] == true {
            return false;
        }

        self.semaphore_need[tid]
            .iter()
            .zip(self.semaphore_available.iter())
            .all(|(&x, &y)| x <= y)
    }

    /// alloc resource to task
    pub fn alloc_semaphores_to_task(&mut self, tid: usize) {
        // set allocation
        for i in 0..self.semaphore_available.len() {
            self.semaphore_allocation[tid][i] += self.semaphore_need[tid][i];
            self.semaphore_available[i] -= self.semaphore_need[tid][i];
            self.semaphore_need[tid][i] = 0;
        }
    }

    /// dealloc resource from task
    pub fn dealloc_semaphore_from_task(&mut self, tid: usize) {
        for i in 0..self.semaphore_available.len() {
            self.semaphore_available[i] += self.semaphore_allocation[tid][i];
            self.semaphore_allocation[tid][i] = 0;
        }
    }

    /// from wait queue pop a task have ennugh semaphore to run
    pub fn pop_task_from_wait_queue(&mut self) -> Option<Arc<TaskControlBlock>> {
        //println!("HaogyKernel pop_task_from_wait_queue: Begin");
        if let Some(num) = self.semaphore_wait_queue
                    .iter()
                    .enumerate()
                    .find(|(_, &ref x)| {self.task_running_has_enough_semaphores(x.as_ref().unwrap().get_tid())})
                    .map(|(i, _)| i) {
            //println!("HaogyKernel pop_task_from_wait_queue: pop task={}", num);
            return self.semaphore_wait_queue.remove(num);    
        } else {
            //println!("Haogy Kernel pop_task_from_wait_queue: None wait_queue.len={:?} need={:?} available={:?}", 
            //    self.semaphore_wait_queue.len(), self.semaphore_need, self.semaphore_available);
            return None;
        }
    } 
    
    /// tids of task in waited list
    pub fn tids_in_waited_list(&self) -> Vec<usize> {
        self.semaphore_wait_queue.clone().iter().map(|x| x.as_ref().unwrap().get_tid()).collect()

    }

    /// test safety status
    pub fn alloc_is_safety(&self, tid: usize) -> bool {
        //println!("Haogy alloc_is_safety BEGIN: tid={}", tid);
        let new_need = self.semaphore_need.to_vec();
        let mut new_available = self.semaphore_available.to_vec();
        let mut new_finish = self.finish.to_vec();
        let mut new_waited_tids: Vec<usize> = self.tids_in_waited_list().to_vec();
        
        //println!("Haogy alloc_is_safety: tid={} new_waited_tids={:?} ", tid, new_waited_tids);
        let tasks_number = new_finish.len();
        let resources_number = new_available.len();
    
        // push self to wait list
        new_waited_tids.push(tid);
        new_waited_tids.sort();
    
        // find  not waited task and dealloc resources
        for i in 0..tasks_number {
            if new_finish[i] == false && new_waited_tids.iter().find(|&&x| x == i).is_none() {
                //println!("Haogy alloc_is_safety: task[{}] is running and add to alloc", i);
                for t in 0..resources_number {
                    new_available[t] += self.semaphore_allocation[i][t];
                }
                new_finish[i] = true;
            }   
        }
    
        loop {
            if new_waited_tids.len() == 0 {
                break;
            }
            
            //println!("Haogy alloc_is_safety before can_running_task: tid={}", tid);
            let mut wait_tid_tuple: Option<(usize, usize)> = None;
            
            for i in 0..new_waited_tids.len() {
                let tid4 = new_waited_tids[i];
                if new_need[tid4].iter().zip(new_available.clone().iter()).all(|(&x, &y)| x <= y) {
                    
                    wait_tid_tuple = Some((i, tid4));
                    break;
                }
            }

            if let Some((serial, wait_tid)) = wait_tid_tuple {
                //println!("Haogy alloc_is_safety: can_pop_task=true: tid={} wait_tid={}", tid, wait_tid);
                // self can run
                if tid == wait_tid {
                    //println!("Haogy alloc_is_safety: return=true tid={} wait_tid={}", tid, wait_tid);
                    return true;
                }

                // pop task2 from waited list
                //println!("Haogy alloc_is_safety: before !remove!: tid={} wait_tid={}", tid, wait_tid);
                new_waited_tids.remove(serial);
                
                // dealloc resources
                for i in 0..resources_number { 
                    //println!("Haogy alloc_is_safety: before !update available!: tid={} wait_tid={}", tid, wait_tid);
                    new_available[i] += self.semaphore_need[wait_tid][i];
                    new_available[i] += self.semaphore_allocation[wait_tid][i];
                }

            } else {
                //println!("Haogy alloc_is_safety: Unsafe! new_waited_tids={:?} |  | new_available={:?}", &new_waited_tids, &new_available);
                return false;
            }
        }
        
        true
    }
    // ****** END xisanlou add at ch8 No.11
}

impl ProcessControlBlock {
    /// inner_exclusive_access
    pub fn inner_exclusive_access(&self) -> RefMut<'_, ProcessControlBlockInner> {
        self.inner.exclusive_access()
    }
    /// new process from elf file
    pub fn new(elf_data: &[u8]) -> Arc<Self> {
        trace!("kernel: ProcessControlBlock::new");
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        // allocate a pid
        let pid_handle = pid_alloc();
        let process = Arc::new(Self {
            pid: pid_handle,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: None,
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: vec![
                        // 0 -> stdin
                        Some(Arc::new(Stdin)),
                        // 1 -> stdout
                        Some(Arc::new(Stdout)),
                        // 2 -> stderr
                        Some(Arc::new(Stdout)),
                    ],
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    // ****** START xisanlou add at ch8 No.2
                    enable_deadlock_detect: false,
                    mutex_allocation: Vec::new(),
                    semaphore_available: Vec::new(),
                    semaphore_allocation: Vec::new(),
                    semaphore_need: Vec::new(),
                    finish: Vec::new(),
                    semaphore_wait_queue: Vec::new(),
                    // ****** END xisanlou add at ch8 No.2
                })
            },
        });
        // create a main thread, we should allocate ustack and trap_cx here
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&process),
            ustack_base,
            true,
        ));
        // prepare trap_cx of main thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        let ustack_top = task_inner.res.as_ref().unwrap().ustack_top();
        let kstack_top = task.kstack.get_top();
        drop(task_inner);
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            ustack_top,
            KERNEL_SPACE.exclusive_access().token(),
            kstack_top,
            trap_handler as usize,
        );
        // add main thread to the process
        let mut process_inner = process.inner_exclusive_access();
        process_inner.tasks.push(Some(Arc::clone(&task)));
        // ****** START xisanlou add at ch8 No.4
        process_inner.finish.push(false);
        // ****** END xisanlou add at ch8 No.4
        drop(process_inner);
        insert_into_pid2process(process.getpid(), Arc::clone(&process));
        // add main thread to scheduler
        add_task(task);
        process
    }

    /// Only support processes with a single thread.
    pub fn exec(self: &Arc<Self>, elf_data: &[u8], args: Vec<String>) {
        trace!("kernel: exec");
        assert_eq!(self.inner_exclusive_access().thread_count(), 1);
        // memory_set with elf program headers/trampoline/trap context/user stack
        trace!("kernel: exec .. MemorySet::from_elf");
        let (memory_set, ustack_base, entry_point) = MemorySet::from_elf(elf_data);
        let new_token = memory_set.token();
        // substitute memory_set
        trace!("kernel: exec .. substitute memory_set");
        self.inner_exclusive_access().memory_set = memory_set;
        // then we alloc user resource for main thread again
        // since memory_set has been changed
        trace!("kernel: exec .. alloc user resource for main thread again");
        let task = self.inner_exclusive_access().get_task(0);
        let mut task_inner = task.inner_exclusive_access();
        task_inner.res.as_mut().unwrap().ustack_base = ustack_base;
        task_inner.res.as_mut().unwrap().alloc_user_res();
        task_inner.trap_cx_ppn = task_inner.res.as_mut().unwrap().trap_cx_ppn();
        // push arguments on user stack
        trace!("kernel: exec .. push arguments on user stack");
        let mut user_sp = task_inner.res.as_mut().unwrap().ustack_top();
        user_sp -= (args.len() + 1) * core::mem::size_of::<usize>();
        let argv_base = user_sp;
        let mut argv: Vec<_> = (0..=args.len())
            .map(|arg| {
                translated_refmut(
                    new_token,
                    (argv_base + arg * core::mem::size_of::<usize>()) as *mut usize,
                )
            })
            .collect();
        *argv[args.len()] = 0;
        for i in 0..args.len() {
            user_sp -= args[i].len() + 1;
            *argv[i] = user_sp;
            let mut p = user_sp;
            for c in args[i].as_bytes() {
                *translated_refmut(new_token, p as *mut u8) = *c;
                p += 1;
            }
            *translated_refmut(new_token, p as *mut u8) = 0;
        }
        // make the user_sp aligned to 8B for k210 platform
        user_sp -= user_sp % core::mem::size_of::<usize>();
        // initialize trap_cx
        trace!("kernel: exec .. initialize trap_cx");
        let mut trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            task.kstack.get_top(),
            trap_handler as usize,
        );
        trap_cx.x[10] = args.len();
        trap_cx.x[11] = argv_base;
        *task_inner.get_trap_cx() = trap_cx;
    }

    /// Only support processes with a single thread.
    pub fn fork(self: &Arc<Self>) -> Arc<Self> {
        trace!("kernel: fork");
        let mut parent = self.inner_exclusive_access();
        assert_eq!(parent.thread_count(), 1);
        // clone parent's memory_set completely including trampoline/ustacks/trap_cxs
        let memory_set = MemorySet::from_existed_user(&parent.memory_set);
        // alloc a pid
        let pid = pid_alloc();
        // copy fd table
        let mut new_fd_table: Vec<Option<Arc<dyn File + Send + Sync>>> = Vec::new();
        for fd in parent.fd_table.iter() {
            if let Some(file) = fd {
                new_fd_table.push(Some(file.clone()));
            } else {
                new_fd_table.push(None);
            }
        }
        // create child process pcb
        let child = Arc::new(Self {
            pid,
            inner: unsafe {
                UPSafeCell::new(ProcessControlBlockInner {
                    is_zombie: false,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: Vec::new(),
                    exit_code: 0,
                    fd_table: new_fd_table,
                    signals: SignalFlags::empty(),
                    tasks: Vec::new(),
                    task_res_allocator: RecycleAllocator::new(),
                    mutex_list: Vec::new(),
                    semaphore_list: Vec::new(),
                    condvar_list: Vec::new(),
                    // ****** START xisanlou add at ch8 No.3
                    enable_deadlock_detect: false,
                    mutex_allocation: Vec::new(),
                    semaphore_available: Vec::new(),
                    semaphore_allocation: Vec::new(),
                    semaphore_need: Vec::new(),
                    finish: Vec::new(),
                    semaphore_wait_queue: Vec::new(),
                    // ****** END xisanlou add at ch8 No.3
                })
            },
        });
        // add child
        parent.children.push(Arc::clone(&child));
        // create main thread of child process
        let task = Arc::new(TaskControlBlock::new(
            Arc::clone(&child),
            parent
                .get_task(0)
                .inner_exclusive_access()
                .res
                .as_ref()
                .unwrap()
                .ustack_base(),
            // here we do not allocate trap_cx or ustack again
            // but mention that we allocate a new kstack here
            false,
        ));
        // attach task to child process
        let mut child_inner = child.inner_exclusive_access();
        child_inner.tasks.push(Some(Arc::clone(&task)));
        // ****** START xisanlou add at ch8 No.5
        child_inner.finish.push(false);
        // ****** END xisanlou add at ch8 No.5
        drop(child_inner);
        // modify kstack_top in trap_cx of this thread
        let task_inner = task.inner_exclusive_access();
        let trap_cx = task_inner.get_trap_cx();
        trap_cx.kernel_sp = task.kstack.get_top();
        drop(task_inner);
        insert_into_pid2process(child.getpid(), Arc::clone(&child));
        // add this thread to scheduler
        add_task(task);
        child
    }
    /// get pid
    pub fn getpid(&self) -> usize {
        self.pid.0
    }
}
