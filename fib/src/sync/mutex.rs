//! One of the most basic synchronization primitives.

use std::{collections::VecDeque, fmt::Debug, ops::{Deref, DerefMut}};

use crate::{runtime::{runtime, wake_task}, task::{packet::Packet, task::TaskState, BlockCause}, utils::STCell};

pub struct Mutex<T> {
    inner: STCell<MutexInner<T>>,
}

impl<T: Debug> Debug for Mutex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mutex")
            .field("data", &self.inner.get().data)
            .field("locked", &self.inner.get().locked)
            .field("waiters", &self.inner.get().waiters)
            .finish()
    }
}

struct MutexInner<T> {
    data: T,
    locked: bool,
    waiters: VecDeque<usize>,
}

pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
}

impl<T> Mutex<T> {
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

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.mutex.inner.get().data
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.mutex.inner.get_mut().data
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        let inner = self.mutex.inner.get_mut();
        inner.locked = false;
        if let Some(waiter_id) = inner.waiters.pop_front() {
            wake_task(waiter_id);
        }
    }
}