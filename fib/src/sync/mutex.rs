//! One of the most basic synchronization primitives.

use std::{collections::VecDeque, ops::{Deref, DerefMut}};

use crate::{runtime::runtime, task::{packet::Packet, task::TaskState, BlockCause}, utils::STCell};

pub struct Mutex<T: 'static> {
    inner: STCell<MutexInner<T>>,
}

struct MutexInner<T: 'static> {
    data: T,
    locked: bool,
    waiters: VecDeque<usize>,
}

pub struct MutexGuard<'a, T: 'static> {
    mutex: &'a Mutex<T>,
}

impl<T: 'static> Mutex<T> {
    pub const fn new(t: T) -> Self {
        let inner = MutexInner {
            data: t,
            locked: false,
            waiters: VecDeque::new(),
        };
        Self { inner: STCell::new(inner) }
    }

    pub fn lock(&self) -> MutexGuard<'_, T> {
        let inner = self.inner.get_mut();
        while inner.locked {
            let mut rt = runtime();
            inner.waiters.push_back(rt.cur_task());
            rt.yield_to_base(Packet::<()>::block_on(BlockCause::Lock));
        }
        inner.locked = true;

        MutexGuard {
            mutex: self,
        }
    }
}

impl<T: 'static> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.mutex.inner.get().data
    }
}

impl<T: 'static> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mutex.inner.get_mut().data
    }
}

impl<T: 'static> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        let inner = self.mutex.inner.get_mut();
        inner.locked = false;
        if let Some(waiter_id) = inner.waiters.pop_front() {
            let mut rt = runtime();
            let mut waiter = rt.blocking_tasks.remove(&waiter_id)
                .expect("No task found for waiter id");
            waiter.trans_state(TaskState::Ready);
            rt.running_tasks.push_back(waiter);
        }
    }
}