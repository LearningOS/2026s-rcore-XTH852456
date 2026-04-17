//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::block_current_and_run_next;
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc, vec::Vec};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
    /// Get current owner tid, if any
    fn owner(&self) -> Option<usize>;
    /// Get tids currently waiting for this mutex
    fn waiters(&self) -> Vec<usize>;
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    owner: UPSafeCell<Option<usize>>,
    wait_queue: UPSafeCell<VecDeque<Arc<TaskControlBlock>>>,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
            owner: unsafe { UPSafeCell::new(None) },
            wait_queue: unsafe { UPSafeCell::new(VecDeque::new()) },
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        let mut locked = self.locked.exclusive_access();
        if *locked {
            let mut wait_queue = self.wait_queue.exclusive_access();
            wait_queue.push_back(current_task().unwrap());
            drop(wait_queue);
            drop(locked);
            block_current_and_run_next();
        } else {
            *locked = true;
            let current_tid = current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid;
            *self.owner.exclusive_access() = Some(current_tid);
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut wait_queue = self.wait_queue.exclusive_access();
        if let Some(waking_task) = wait_queue.pop_front() {
            let waking_tid = waking_task.inner_exclusive_access().res.as_ref().unwrap().tid;
            *self.owner.exclusive_access() = Some(waking_tid);
            wakeup_task(waking_task);
        } else {
            *self.owner.exclusive_access() = None;
            let mut locked = self.locked.exclusive_access();
            *locked = false;
        }
    }

    fn owner(&self) -> Option<usize> {
        *self.owner.exclusive_access()
    }

    fn waiters(&self) -> Vec<usize> {
        self.wait_queue
            .exclusive_access()
            .iter()
            .filter_map(|task| task.inner_exclusive_access().res.as_ref().map(|res| res.tid))
            .collect()
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
}

pub struct MutexBlockingInner {
    locked: bool,
    owner: Option<usize>,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    owner: None,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
            mutex_inner.owner = Some(current_task().unwrap().inner_exclusive_access().res.as_ref().unwrap().tid);
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            mutex_inner.owner = Some(waking_task.inner_exclusive_access().res.as_ref().unwrap().tid);
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
            mutex_inner.owner = None;
        }
    }

    fn owner(&self) -> Option<usize> {
        self.inner.exclusive_access().owner
    }

    fn waiters(&self) -> Vec<usize> {
        self.inner
            .exclusive_access()
            .wait_queue
            .iter()
            .filter_map(|task| task.inner_exclusive_access().res.as_ref().map(|res| res.tid))
            .collect()
    }
}
