//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
// ****** START xisanlou add at ch6 No.1
use crate::fs::{link_at, unlink_at};
use core::{mem::size_of, slice::from_raw_parts};
//use core::arch::asm;
// ****** END xisanlou add at ch6 No.1

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    // ****** START xisanlou add at ch6 No.2
    trace!("kernel:pid[{}] sys_fstat", current_task().unwrap().pid.0);
    
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if _fd >= inner.fd_table.len() {
        //println!("HAOGY kernel: sys_fstat retrun -1 [fd >= fd_table.len]");
        return -1;
    }
    
    if let Some(file) = &inner.fd_table[_fd] {
        let file = file.clone();
        
        // release current task TCB manually to avoid multi-borrow
        drop(inner);

        
        if let Some(file_stat) = file.get_fstat() {
            // Copy Stat to user space
            let buffers = translated_byte_buffer(
                current_user_token(),
                _st as *const u8,
                size_of::<Stat>(),
            );
            if buffers.len() == 0 {
                //println!("HAOGY kernel: sys_fstat retrun -1 [buffers.len() == 0]");
                return -1;
            }

            let mut offset = 0;
            for buffer in buffers {
                unsafe {
                    let src = from_raw_parts((&file_stat as *const Stat as *const u8).wrapping_add(offset), buffer.len());
                    buffer.copy_from_slice(src);
                }
                offset += buffer.len();
            }
            //unsafe {
            //    asm!("fence.i");
            //}
        } else {
            //println!("HAOGY kernel: sys_fstat retrun -1 [file.get_fstat() return None]");
            return -1;
        }

        0
    } else {
        //println!("HAOGY kernel: sys_fstat retrun -1 [&inner.fd_table[_fd] return None]");
        -1
    }
    // ****** END xisanlou add at ch6 No.2
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    // ****** START xisanlou add at ch6 No.3
    trace!("kernel:pid[{}] sys_linkat", current_task().unwrap().pid.0);
    
    let token = current_user_token();
    let old_name = translated_str(token, _old_name);
    let new_name = translated_str(token, _new_name);
    if old_name == new_name {
        return -1;
    }

    link_at(old_name.as_str(), new_name.as_str())
    // ****** END xisanlou add at ch6 No.3
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(_name: *const u8) -> isize {
    // ****** START xisanlou add at ch6 No.4
    trace!(
        "kernel:pid[{}] sys_unlinkat ",
        current_task().unwrap().pid.0
    );
    
    let token = current_user_token();
    let name = translated_str(token, _name);

    unlink_at(name.as_str())
    // ****** END xisanlou add at ch6 No.4
}
