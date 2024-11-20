use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
// ****** START xisanlou add at ch8 No.7
use alloc::vec;
use crate::task::wakeup_task;
// ****** START xisanlou add at ch8 No.7
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        // ****** START xisanlou add at ch8 No.1
        if process_inner.mutex_allocation.len() != 0 {
            for row in process_inner.mutex_allocation.iter_mut() {
                row.push(0);
            } 
        } else {
            for _ in 0..(process_inner.tasks.len()) {
                process_inner.mutex_allocation.push(vec![0]);
            }        
        }
        // ****** END xisanlou add at ch8 No.1
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    // ****** START xisanlou add at ch8 No.2
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid
    );
    // ****** END xisanlou add at ch8 No.2
    let process = current_process();
    // ****** START xisanlou add at ch8 No.3
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_deadlock_detect == true {
        let allocated = process_inner
            .mutex_allocation
            .iter()
            .fold(0, |sum, row| sum + row[mutex_id]);
        if allocated >= 1 {
           return -isize::from_str_radix("DEAD", 16).unwrap();
        } else {
            process_inner.mutex_allocation[tid][mutex_id] = 1;
        }
    }
    // ****** END xisanlou add at ch8 No.3
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    // ****** START xisanlou add at ch8 No.4
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );
    // ****** END xisanlou add at ch8 No.4
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    // ****** START xisanlou add at ch8 No.5
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    if process_inner.enable_deadlock_detect == true {
        process_inner.mutex_allocation[tid][mutex_id] = 0;
    }
    // ****** END xisanlou add at ch8 No.5
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));

        // ****** START xisanlou add at ch8 No.20
        process_inner.semaphore_available[id] = res_count as isize;
        for r in process_inner.semaphore_allocation.iter_mut() {
            r[id] = 0;
        }

        for r in process_inner.semaphore_need.iter_mut() {
            r[id] = 0;
        }
        // ****** END xisanlou add at ch8 No.20

        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        
        // ****** START xisanlou add at ch8 No.10
        process_inner.semaphore_available.push(res_count as isize);
        if process_inner.semaphore_allocation.len() == 0 {
            // first init allocation
            for _ in 0..process_inner.tasks.len() {
                process_inner.semaphore_allocation.push(vec![0]);
            }
        } else {
            for row in process_inner.semaphore_allocation.iter_mut() {
                row.push(0);
            }
        }

        if process_inner.semaphore_need.len() == 0 {
            // first init need
            for _ in 0..process_inner.tasks.len() {
                process_inner.semaphore_need.push(vec![0]);
            }
        } else {
            for row in process_inner.semaphore_need.iter_mut() {
                row.push(0);
            }
        }
        // ****** END xisanlou add at ch8 No.10
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    // ****** START xisanlou add at ch8 No.14
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;

    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );
    // ****** END xisanlou add at ch8 No.14
    let process = current_process();
    // ****** START xisanlou add at ch8 No.11
    //println!("Haogy Kernel SEM_UP tid={} sem_id={} Begin", tid, sem_id);
    let process_inner = process.inner_exclusive_access();
    let enable_deadlock_detect = process_inner.enable_deadlock_detect;
    drop(process_inner);

    if enable_deadlock_detect {
        let mut process_inner = process.inner_exclusive_access();
        process_inner.semaphore_available[sem_id] += 1;
        
        let option_task2 = process_inner.pop_task_from_wait_queue();
        drop(process_inner);

        if let Some(task2) = option_task2 {
            let task2_tid = task2.get_tid();
            //println!("Haogy Kernel SEM_UP tid={} sem_id={} wakeup_task={}", tid, sem_id, task2_tid);
            let mut process_inner = process.inner_exclusive_access();
            process_inner.alloc_semaphores_to_task(task2_tid);
            drop(process_inner);
            wakeup_task(task2);
        }
    } else { 
        let process_inner = process.inner_exclusive_access();   
        let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
        drop(process_inner);     
        sem.up();
    }
    // ****** END xisanlou add at ch8 No.11
    //println!("Haogy Kernel SEM_UP END tid={} sem_id={}", tid, sem_id);
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    // ****** START xisanlou add at ch8 No.12
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        tid,
    );
    // ****** END xisanlou add at ch8 No.12
    let process = current_process();

    // ****** START xisanlou add at ch8 No.13
    //println!("Haogy Kernel SEM_DOWN tid={} sem_id={} Begin", tid, sem_id);
    let process_inner = process.inner_exclusive_access();
    let enable_deadlock_detect = process_inner.enable_deadlock_detect;
    drop(process_inner);
    
    if enable_deadlock_detect == true {
        //println!("Haogy Kernel SEM_DOWN tid={} sem_id={} enable_deadlock_detect==TRUE", tid, sem_id);
        // self task has enough semaphores to continue running
        let mut process_inner = process.inner_exclusive_access();
        process_inner.semaphore_need[tid][sem_id] += 1;
        let self_can_continue = process_inner.semaphore_need[tid][sem_id] <= process_inner.semaphore_available[sem_id]; 
        drop(process_inner);
        if self_can_continue {
            let mut process_inner = process.inner_exclusive_access();
            process_inner.alloc_semaphores_to_task(tid);
            return 0;
        }

        let process_inner = process.inner_exclusive_access();
        let is_safety = process_inner.alloc_is_safety(tid);
        //println!("Haogy Kernel SEM_DOWN tid={} sem_id={} alloc_is_safety return", tid, sem_id);
        drop(process_inner);

        // test security
        if is_safety {
            if tid != 0 {
                let mut process_inner = process.inner_exclusive_access();
                process_inner.semaphore_wait_queue.push(current_task());
                drop(process_inner);
                block_current_and_run_next();
                return 0;
            } else {
                let mut process_inner = process.inner_exclusive_access();
                process_inner.alloc_semaphores_to_task(0);
                return 0;
            }

        } else {
            let mut process_inner = process.inner_exclusive_access();
            //println!("Haogy Kernel SEM_DOWN END !!deadlock!! tid={} sem_id={} need={:?} available={:?} alloc={:?} finish={:?} wait_queue.len={:?}", tid, sem_id, 
            //        process_inner.semaphore_need,
            //        process_inner.semaphore_available,
            //        process_inner.semaphore_allocation,
            //        process_inner.finish,
            //        process_inner.semaphore_wait_queue.len(),
            //);
            process_inner.semaphore_need[tid][sem_id] -= 1;
            return -0xdead;
        }  
    } 
    // ****** END xisanlou add at ch8 No.13

    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();
    //println!("Haogy Kernel DOWN END tid={} sem_id={}", tid, sem_id);
    return 0;
}

/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    // ****** START xisanlou add at ch8 No.6
    trace!(
        "kernel:pid[{}] sys_enable_deadlock_detect",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
    );

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    match _enabled {
        0 => process_inner.enable_deadlock_detect = false,
        1 => process_inner.enable_deadlock_detect = true,
        _ => return -1,
    };

    0
    // ****** END xisanlou add at ch8 No.6
}




