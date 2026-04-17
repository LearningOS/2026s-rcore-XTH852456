use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::{sync::Arc, vec, vec::Vec};

const DEADLOCK_ERR: isize = -0xDEAD;

fn current_tid() -> usize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid
}

fn task_index(task_tids: &[usize], tid: usize) -> Option<usize> {
    task_tids.iter().position(|task_tid| *task_tid == tid)
}

fn is_safe_state(
    available: Vec<usize>,
    allocation: Vec<Vec<usize>>,
    need: Vec<Vec<usize>>,
) -> bool {
    let mut work = available;
    let mut finish = vec![false; allocation.len()];

    loop {
        let mut progress = false;
        for i in 0..allocation.len() {
            if finish[i] {
                continue;
            }
            if need[i]
                .iter()
                .zip(work.iter())
                .all(|(need, available)| *need <= *available)
            {
                for j in 0..work.len() {
                    work[j] += allocation[i][j];
                }
                finish[i] = true;
                progress = true;
            }
        }
        if !progress {
            break;
        }
    }

    finish.into_iter().all(|finished| finished)
}

macro_rules! mutex_deadlock_safe {
    ($process:expr, $pending_mutex_id:expr) => {{
        let (task_tids, mutexes) = {
            let process_inner = $process.inner_exclusive_access();
            let task_tids = process_inner
                .tasks
                .iter()
                .filter_map(|task| task.as_ref().cloned())
                .collect::<Vec<_>>();
            let task_tids = task_tids
                .iter()
                .filter_map(|task| task.inner_exclusive_access().res.as_ref().map(|res| res.tid))
                .collect::<Vec<_>>();
            let mutexes = process_inner
                .mutex_list
                .iter()
                .enumerate()
                .filter_map(|(id, mutex)| mutex.as_ref().cloned().map(|mutex| (id, mutex)))
                .collect::<Vec<_>>();
            (task_tids, mutexes)
        };

        let task_count = task_tids.len();
        let resource_count = mutexes.len();
        if task_count == 0 || resource_count == 0 {
            true
        } else {
            let mut available = vec![1usize; resource_count];
            let mut allocation = vec![vec![0usize; resource_count]; task_count];
            let mut need = vec![vec![0usize; resource_count]; task_count];
            let mut valid = true;

            for (column, (mutex_id, mutex)) in mutexes.iter().enumerate() {
                if let Some(owner) = mutex.owner() {
                    if let Some(row) = task_index(&task_tids, owner) {
                        allocation[row][column] = 1;
                        available[column] = 0;
                    }
                }
                for waiter in mutex.waiters() {
                    if let Some(row) = task_index(&task_tids, waiter) {
                        need[row][column] = 1;
                    }
                }
                if $pending_mutex_id == Some(*mutex_id) {
                    if let Some(row) = task_index(&task_tids, current_tid()) {
                        need[row][column] = 1;
                    } else {
                        valid = false;
                        break;
                    }
                }
            }

            valid && is_safe_state(available, allocation, need)
        }
    }};
}

macro_rules! semaphore_deadlock_safe {
    ($process:expr, $pending_sem_id:expr) => {{
        let (task_tids, semaphores) = {
            let process_inner = $process.inner_exclusive_access();
            let task_tids = process_inner
                .tasks
                .iter()
                .filter_map(|task| task.as_ref().cloned())
                .collect::<Vec<_>>();
            let task_tids = task_tids
                .iter()
                .filter_map(|task| task.inner_exclusive_access().res.as_ref().map(|res| res.tid))
                .collect::<Vec<_>>();
            let semaphores = process_inner
                .semaphore_list
                .iter()
                .enumerate()
                .filter_map(|(id, sem)| sem.as_ref().cloned().map(|sem| (id, sem)))
                .collect::<Vec<_>>();
            (task_tids, semaphores)
        };

        let task_count = task_tids.len();
        let resource_count = semaphores.len();
        if task_count == 0 || resource_count == 0 {
            true
        } else {
            let mut available = vec![0usize; resource_count];
            let mut allocation = vec![vec![0usize; resource_count]; task_count];
            let mut need = vec![vec![0usize; resource_count]; task_count];
            let mut valid = true;

            for (column, (sem_id, sem)) in semaphores.iter().enumerate() {
                available[column] = sem.available();
                for (holder_tid, holder_count) in sem.holders() {
                    if let Some(row) = task_index(&task_tids, holder_tid) {
                        allocation[row][column] = holder_count;
                    }
                }
                for waiter in sem.waiters() {
                    if let Some(row) = task_index(&task_tids, waiter) {
                        need[row][column] = 1;
                    }
                }
                if $pending_sem_id == Some(*sem_id) {
                    if let Some(row) = task_index(&task_tids, current_tid()) {
                        need[row][column] = 1;
                    } else {
                        valid = false;
                        break;
                    }
                }
            }

            valid && is_safe_state(available, allocation, need)
        }
    }};
}
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
        process_inner.mutex_list.len() as isize - 1
    }
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
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
    let mutex = {
        let process_inner = process.inner_exclusive_access();
        match process_inner.mutex_list.get(mutex_id).and_then(|mutex| mutex.as_ref()) {
            Some(mutex) => Arc::clone(mutex),
            None => return -1,
        }
    };
    if process.inner_exclusive_access().deadlock_detect && mutex.owner().is_some() {
        if !mutex_deadlock_safe!(process, Some(mutex_id)) {
            return DEADLOCK_ERR;
        }
    }
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
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
    let mutex = {
        let process_inner = process.inner_exclusive_access();
        match process_inner.mutex_list.get(mutex_id).and_then(|mutex| mutex.as_ref()) {
            Some(mutex) => Arc::clone(mutex),
            None => return -1,
        }
    };
    drop(process);
    mutex.unlock();
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
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
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
    let sem = {
        let process_inner = process.inner_exclusive_access();
        match process_inner.semaphore_list.get(sem_id).and_then(|sem| sem.as_ref()) {
            Some(sem) => Arc::clone(sem),
            None => return -1,
        }
    };
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
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
    let sem = {
        let process_inner = process.inner_exclusive_access();
        match process_inner.semaphore_list.get(sem_id).and_then(|sem| sem.as_ref()) {
            Some(sem) => Arc::clone(sem),
            None => return -1,
        }
    };
    if process.inner_exclusive_access().deadlock_detect && sem.available() == 0 {
        if !semaphore_deadlock_safe!(process, Some(sem_id)) {
            return DEADLOCK_ERR;
        }
    }
    sem.down();
    0
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
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    if enabled != 0 && enabled != 1 {
        return -1;
    }

    let process = current_process();
    if enabled == 1 {
        let mutex_safe = mutex_deadlock_safe!(process, None);
        let sem_safe = semaphore_deadlock_safe!(process, None);
        if !mutex_safe || !sem_safe {
            return -1;
        }
    }

    process.inner_exclusive_access().deadlock_detect = enabled == 1;
    0
}
