use std::{cell::RefCell, collections::VecDeque, ops::{Deref, DerefMut}};

use crate::{runtime::{runtime, wake_task}, task::packet::Packet, utils::STCell};


pub struct RwLock<T> {
    inner: STCell<RwLockInner<T>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RwLockState {
    None,
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Access {
    Read,
    Write,
}

struct RwLockInner<T> {
    data: Box<T>,
    state: RwLockState,
    waiters: VecDeque<(usize, Access)>,
    reader_count: usize,
}

impl<T> RwLock<T> {
    pub fn new(data: T) -> Self
    {
        let inner = RwLockInner {
            data: Box::new(data),
            state: RwLockState::None,
            waiters: VecDeque::new(),
            reader_count: 0,
        };
        Self {
            inner: STCell::new(inner),
        }
    }

    pub fn read(&self)  -> RwLockReadGuard<'_, T> {
        let mut inner = self.inner.get_mut();
        while inner.state == RwLockState::Write {
            let rt = runtime();
            inner.waiters.push_back((rt.cur_task(), Access::Read));
            rt.yield_to_base(Packet::<()>::block_on(crate::task::BlockCause::Lock));
        }
        inner.state = RwLockState::Read;
        inner.reader_count += 1;

        RwLockReadGuard { rwlock: self }
    }

    pub fn write(&self) -> RwLockWriteGuard<'_, T> {
        let mut inner = self.inner.get_mut();
        while inner.state != RwLockState::None {
            let rt = runtime();
            inner.waiters.push_back((rt.cur_task(), Access::Write));
            rt.yield_to_base(Packet::<()>::block_on(crate::task::BlockCause::Lock));
        }
        assert!(inner.reader_count == 0);
        inner.state = RwLockState::Write;

        RwLockWriteGuard { rwlock: self }
    }

}

impl<T> RwLockInner<T> {
    fn wake_up(&mut self) {
        assert!(self.state == RwLockState::None);
        let mut next_access;
        if let Some((_, access)) = self.waiters.front() {
            next_access = *access;
        } else {
            return;
        }
        match next_access {
            Access::Write => {
                // self.state = RwLockState::Write;
                let (task_id, _) = self.waiters.pop_front().unwrap();
                wake_task(task_id);
            },
            Access::Read => {
                // self.state = RwLockState::Read;
                while let Some((task_id, access)) = self.waiters.pop_front() {
                    if access == Access::Read {
                        wake_task(task_id);
                    } else {
                        self.waiters.push_front((task_id, access));
                        break;
                    }
                }
            }
        }
    }
}

pub struct RwLockReadGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<'a, T> Deref for RwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.rwlock.inner.get().data
    }
}

impl<'a, T> Drop for RwLockReadGuard<'a, T> {
    fn drop(&mut self) {
        let inner = self.rwlock.inner.get_mut();
        inner.reader_count -= 1;
        if inner.reader_count == 0 {
            inner.state = RwLockState::None;
            inner.wake_up();   
        }
    }
}

pub struct RwLockWriteGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<'a, T> Drop for RwLockWriteGuard<'a, T> {
    fn drop(&mut self) {
        let inner = self.rwlock.inner.get_mut();
        inner.state = RwLockState::None;
        inner.wake_up();
    }
}

impl<'a, T> Deref for RwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.rwlock.inner.get().data
    }
}

impl<'a, T> DerefMut for RwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rwlock.inner.get_mut().data
    }
}