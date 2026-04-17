//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc};
use alloc::vec::Vec;

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
    pub holders: Vec<(usize, usize)>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                    holders: Vec::new(),
                })
            },
        }
    }

    fn add_holder(holders: &mut Vec<(usize, usize)>, tid: usize) {
        if let Some((_, count)) = holders.iter_mut().find(|(holder_tid, _)| *holder_tid == tid) {
            *count += 1;
        } else {
            holders.push((tid, 1));
        }
    }

    fn remove_holder(holders: &mut Vec<(usize, usize)>, tid: usize) {
        if let Some(index) = holders.iter().position(|(holder_tid, _)| *holder_tid == tid) {
            if holders[index].1 > 1 {
                holders[index].1 -= 1;
            } else {
                holders.remove(index);
            }
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        let current_tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
        Self::remove_holder(&mut inner.holders, current_tid);
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                let waking_tid = task.inner_exclusive_access().res.as_ref().unwrap().tid;
                Self::add_holder(&mut inner.holders, waking_tid);
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        } else {
            let current_tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
            Self::add_holder(&mut inner.holders, current_tid);
        }
    }

    /// Get current holders and their allocation counts
    pub fn holders(&self) -> Vec<(usize, usize)> {
        self.inner.exclusive_access().holders.clone()
    }

    /// Get tids currently waiting for this semaphore
    pub fn waiters(&self) -> Vec<usize> {
        self.inner
            .exclusive_access()
            .wait_queue
            .iter()
            .filter_map(|task| task.inner_exclusive_access().res.as_ref().map(|res| res.tid))
            .collect()
    }

    /// Get currently available resource count
    pub fn available(&self) -> usize {
        self.inner.exclusive_access().count.max(0) as usize
    }
}
