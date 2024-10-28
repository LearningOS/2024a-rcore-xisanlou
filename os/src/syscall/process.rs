//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
        get_current_task_start_time, 
        get_current_task_syscall_times,
        current_user_token,
        current_user_insert_framed_area,
        current_user_vpn_no_overlap,
        current_user_unmap_user_area,
    },
    timer::get_time_us,
    mm::{translated_byte_buffer, VirtAddr, MapPermission},
};
use core::{mem::size_of, slice::from_raw_parts};
//use core::arch::asm;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let buffers = translated_byte_buffer(
        current_user_token(),
        _ts as *const u8,
        size_of::<TimeVal>(),
    );
    if buffers.len() == 0 {
        return -1;
    }

    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    let mut offset = 0;
    for buffer in buffers {
        unsafe {
            let src = from_raw_parts((&time_val as *const TimeVal as *const u8).wrapping_add(offset), buffer.len());
            buffer.copy_from_slice(src);
        }
        offset += buffer.len();
    }
    //unsafe {
    //    asm!("fence.i");
    //}
    
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info ");
    let buffers = translated_byte_buffer(
        current_user_token(),
        _ti as *const u8,
        size_of::<TaskInfo>(),
    );
    if buffers.len() == 0 {
        return -1;
    }

    let syscall_times = get_current_task_syscall_times();
    let start_time = get_current_task_start_time();
     
    let us = get_time_us();
    let sec = us / 1_000_000;
    let usec = us % 1_000_000;
    let time = ((sec & 0xffff) * 1000 + usec / 1000) as usize - start_time;

    let task_info = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: syscall_times,
        time: time,
    };

    let mut offset = 0;
    for buffer in buffers {
        unsafe {
            let src = from_raw_parts((&task_info as *const TaskInfo as *const u8).wrapping_add(offset), buffer.len());
            buffer.copy_from_slice(src);
        }
        offset += buffer.len();
    }
    //unsafe {
    //    asm!("fence.i");
    //}
    
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    // get start and end Virtual addrsss.
    let start_va = VirtAddr::from(_start);
    if ! start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr::from(_start + _len);

    // Test port and change it to MapPermission.
    if ((_port & !0x7) != 0) || ((_port & 0x7) == 0) {
        return -1;
    }

    let mut map_perm = MapPermission::from_bits((_port as u8) << 1).unwrap();
    map_perm.set(MapPermission::U, true);
    
    // Test virtual address range overlapping.
    if ! current_user_vpn_no_overlap(start_va, end_va) {
        return -1;
    }

    // map virtual address to physical address.
    current_user_insert_framed_area(start_va, end_va, map_perm);

    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap ");
    // get start and end Virtual addrsss.
    let start_va = VirtAddr::from(_start);
    if ! start_va.aligned() {
        return -1;
    }
    let end_va = VirtAddr::from(_start + _len);
    if ! end_va.aligned() {
        return -1;
    }

    current_user_unmap_user_area(start_va, end_va)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
