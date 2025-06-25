use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use crate::{runtime::{runtime, wake_task}, task::packet::Packet};


struct SemaphoreCore {
    permits: usize,
    waiters: VecDeque<usize>,
    closed: bool,
}

pub struct Semaphore {
    core: Rc<RefCell<SemaphoreCore>>,
}

pub struct SemaphorePermit<'a> {
    sem: &'a Semaphore,
    permits: usize,
}

#[derive(Debug)]
pub struct AcquireError;

#[derive(Debug)]
pub enum TryAcquireError {
    Closed,
    NoPermits,
}

impl Semaphore {
    pub const MAX_PERMITS: usize = 65535;

    pub fn new(permits: usize) -> Self {
        assert!(permits > 0 && permits <= Self::MAX_PERMITS, "Permits must be between 1 and {}", Self::MAX_PERMITS);
        Self {
            core: Rc::new(RefCell::new(SemaphoreCore {
                permits,
                waiters: VecDeque::new(),
                closed: false,
            })),
        }
    }

    pub fn available_permits(&self) -> usize {
        self.core.borrow().permits
    }

    pub fn add_permits(&self, permits: usize) {
        assert!(self.available_permits() + permits <= Self::MAX_PERMITS,
            "Cannot add more than {} permits", Self::MAX_PERMITS);
        
        if permits == 0 {
            return;
        }
        
        let mut core = self.core.borrow_mut();
        core.permits += permits;
        while core.permits > 0 && !core.waiters.is_empty() {
            let waiter = core.waiters.pop_front().unwrap();
            wake_task(waiter);
            core.permits -= 1;
        }
    }

    /// Forget the specified number of permits, returning the number of permits that were actually forgotten.
    pub fn forget_permits(&self, permits: usize) -> usize {
        let mut core = self.core.borrow_mut();
        if permits == 0 || core.permits == 0 {
            return 0;
        }

        let actual_forgotten = core.permits.min(permits);
        core.permits -= actual_forgotten;

        actual_forgotten
    }

    pub fn acquire(&self) -> Result<SemaphorePermit<'_>, AcquireError> {
        let mut core = self.core.borrow_mut();
        if core.closed {
            return Err(AcquireError);
        }
        if core.permits > 0 {
            core.permits -= 1;
            return Ok(SemaphorePermit { sem: self, permits: 1 })
        }
        let rt = runtime();
        core.waiters.push_back(rt.cur_task());
        drop(core);
        rt.yield_to_base(Packet::<()>::block_on(crate::task::BlockCause::Semaphore));

        core = self.core.borrow_mut();
        if core.closed {
            return Err(AcquireError);
        }
        assert!(core.permits > 0, "Semaphore permits should be available after yielding");
        core.permits -= 1;
        Ok(SemaphorePermit { sem: self, permits: 1 })
    }

    pub fn try_acquire(&self) -> Result<SemaphorePermit<'_>, TryAcquireError> {
        let mut core = self.core.borrow_mut();
        if core.closed {
            return Err(TryAcquireError::Closed);
        }
        if core.permits > 0 {
            core.permits -= 1;
            return Ok(SemaphorePermit { sem: self, permits: 1 });
        }
        Err(TryAcquireError::NoPermits)
    }

    pub fn is_closed(&self) -> bool {
        self.core.borrow().closed
    }

    pub fn close(&self) {
        let mut core = self.core.borrow_mut();
        if core.closed {
            return;
        }
        core.closed = true;

        while let Some(waiter) = core.waiters.pop_front() {
            wake_task(waiter);
        }
    }
}

impl SemaphorePermit<'_> {
    pub fn num_permits(&self) -> usize {
        self.permits
    }

    pub fn forget(mut self) {
        self.permits = 0;
    }

    pub fn merge(&mut self, mut other: Self) {
        assert!(Rc::ptr_eq(&self.sem.core, &other.sem.core), 
            "Cannot merge permits from different semaphores");
        self.permits += other.permits;
        other.permits = 0;
    }

    pub fn split(&mut self, n: usize) -> Option<Self> {
        let n = u16::try_from(n).ok()?;
        let n = n as usize;
        
        if n  > self.permits {
            return None;
        }

        self.permits -= n;

        Some(Self {
            sem: self.sem,
            permits: n as usize,
        })
    }
}

impl Drop for SemaphorePermit<'_> {
    fn drop(&mut self) {
        self.sem.add_permits(self.permits);
    }
}
