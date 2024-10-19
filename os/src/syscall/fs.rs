//! File and filesystem-related syscalls

use core::ptr::copy;
use crate::mm::translated_byte_buffer;
use crate::task::current_user_token;

const FD_STDOUT: usize = 1;

/// write buf of length `len`  to a file with `fd`
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel: sys_write");
    match fd {
        FD_STDOUT => {
            let buffers = translated_byte_buffer(current_user_token(), buf, len);
            for buffer in buffers {
                print!("{}", core::str::from_utf8(buffer).unwrap());
            }
            len as isize
        }
        _ => {
            panic!("Unsupported fd in sys_write!");
        }
    }
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