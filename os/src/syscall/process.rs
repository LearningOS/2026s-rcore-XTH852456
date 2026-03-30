//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::mm::{translated_byte_buffer_checked, MapPermission};
use crate::task::{
    change_program_brk, current_task_mmap, current_task_munmap, current_task_syscall_count,
    current_user_token, exit_current_and_run_next, suspend_current_and_run_next,
};
use crate::timer::get_time_us;
use core::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
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
    let ts = _ts;
    let token = current_user_token();
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let src = unsafe {
        core::slice::from_raw_parts((&time_val as *const TimeVal) as *const u8, size_of::<TimeVal>())
    };
    let mut dst = match translated_byte_buffer_checked(token, ts as *const u8, src.len(), true) {
        Some(v) => v,
        None => return -1,
    };
    let mut offset = 0usize;
    for seg in dst.iter_mut() {
        let seg_len = seg.len();
        seg.copy_from_slice(&src[offset..offset + seg_len]);
        offset += seg_len;
    }
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    let trace_request = _trace_request;
    let id = _id;
    let data = _data;
    let token = current_user_token();
    match trace_request {
        0 => {
            if let Some(buf) = translated_byte_buffer_checked(token, id as *const u8, 1, false)
            {
                buf[0][0] as isize
            } else {
                -1
            }
        }
        1 => {
            if let Some(mut buf) = translated_byte_buffer_checked(token, id as *const u8, 1, true) {
                buf[0][0] = data as u8;
                0
            } else {
                -1
            }
        }
        2 => current_task_syscall_count(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    let start = _start;
    let len = _len;
    let prot = _port;
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if prot & !0x7 != 0 || prot & 0x7 == 0 {
        return -1;
    }
    if start.checked_add(len).is_none() {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let mut map_perm = MapPermission::U;
    if prot & 0x1 != 0 {
        map_perm |= MapPermission::R;
    }
    if prot & 0x2 != 0 {
        map_perm |= MapPermission::W;
    }
    if prot & 0x4 != 0 {
        map_perm |= MapPermission::X;
    }
    if current_task_mmap(start, len, map_perm) {
        0
    } else {
        -1
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    let start = _start;
    let len = _len;
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if start.checked_add(len).is_none() {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    if current_task_munmap(start, len) {
        0
    } else {
        -1
    }
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
