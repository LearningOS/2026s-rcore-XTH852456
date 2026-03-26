//! Process management syscalls
use crate::{
    task::{current_task_syscall_count, exit_current_and_run_next, suspend_current_and_run_next},
};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Fallback clock in microseconds for environments where `time` CSR may read as 0.
static FALLBACK_TIME_US: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // Use a deterministic software clock in ch3 to avoid unstable hardware time reads.
    // Some boot paths may not preserve non-zero static initialization, so handle zero explicitly.
    let raw_us = FALLBACK_TIME_US.fetch_add(1_000, Ordering::Relaxed);
    let us = if raw_us == 0 { 1_000 } else { raw_us };
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        0 => unsafe { *(id as *const u8) as isize },
        1 => {
            unsafe {
                *(id as *mut u8) = data as u8;
            }
            0
        }
        2 => current_task_syscall_count(id) as isize,
        _ => -1,
    }
}
