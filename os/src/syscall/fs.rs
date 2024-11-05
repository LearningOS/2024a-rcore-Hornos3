//! File and filesystem-related syscalls

use alloc::borrow::ToOwned;
use core::intrinsics::copy;
use core::mem::size_of;
use crate::fs::{open_file, OpenFlags, Stat, StatMode, ROOT_INODE};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some((file, _)) = &inner.fd_table[fd] {
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
    if let Some((file, _)) = &inner.fd_table[fd] {
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
        inner.fd_table[fd] = Some((inode, path));
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
    trace!("kernel:pid[{}] sys_fstat", current_task().unwrap().pid.0);

    let ct = current_task().unwrap();
    let inner = ct.inner_exclusive_access();
    if inner.fd_table.len() <= _fd {
        println!("Invalid fd: {}", _fd);
        return -1;
    }

    let mut st: Stat = Stat::new();

    if let Some((_, name)) = &inner.fd_table[_fd] {
        let name = name.to_owned();
        drop(inner);
        let inode = ROOT_INODE.find(&name).unwrap();
        st.ino = inode.get_inode_id();
        st.mode = if inode.is_file() { StatMode::FILE } else { StatMode::DIR };
        st.nlink = ROOT_INODE.get_hard_link_count(name);
        copy_to_current_user(_st, &st as *const Stat, size_of::<Stat>());
    } else {
        println!("Invalid fd: {}", _fd);
        return -1;
    }

    0
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_linkat", current_task().unwrap().pid.0);

    let old_name = translated_str(current_user_token(), _old_name);
    let new_name = translated_str(current_user_token(), _new_name);

    println!("linkat: {}", new_name);

    if old_name == new_name {
        println!("Cannot linkat for a same file name");
        return -1;
    }

    let file_list = ROOT_INODE.ls();
    if file_list.iter().any(|x| x == &new_name) {
        println!("Cannot linkat for a existed name: {}", new_name);
        return -1;
    }

    if let Some(inode) = ROOT_INODE.find(&old_name) {
        assert!(inode.create_link_unchecked(&ROOT_INODE, &new_name))
    } else {
        println!("linkat: original file {} not found", old_name);
        return -1;
    }

    0
}

/// TODO
/// YOUR JOB: Implement unlinkat.
/// how to work: set the inode id into u32::MAX and clear the name
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_unlinkat", current_task().unwrap().pid.0);
    let name = translated_str(current_user_token(), _name);

    if let Some(_) = ROOT_INODE.find(&name) {
        ROOT_INODE.unlink(&name);
    } else {
        println!("unlinkat: file {} not found", name);
        return -1;
    }

    0
}

#[allow(unused)]
pub fn copy_to_current_user<T>(user_buf: *mut T, kern_buf: *const T, len: usize) -> isize {
    let buffers = translated_byte_buffer(current_user_token(), user_buf as *const u8, len);
    for buffer in buffers {
        unsafe {
            copy(kern_buf as *const u8, buffer.as_mut_ptr(), buffer.len());
        }
    }
    len as isize
}

#[allow(unused)]
pub fn copy_from_current_user<T>(kern_buf: *mut T, user_buf: *const T, len: usize) -> isize {
    let buffers = translated_byte_buffer(current_user_token(), user_buf as *const u8, len);
    for buffer in buffers {
        unsafe {
            copy(buffer.as_mut_ptr(), kern_buf as *mut u8, buffer.len());
        }
    }
    len as isize
}