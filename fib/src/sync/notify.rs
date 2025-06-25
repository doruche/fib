use std::{cell::RefCell, collections::{VecDeque}, rc::Rc};

use crate::{runtime::{runtime, wake_task}, sync::notify, task::packet::Packet};

struct NotifyCore {
    waiters: VecDeque<usize>,
    permit: Option<()>,
}

#[derive(Clone)]
pub struct Notify {
    core: Rc<RefCell<NotifyCore>>,
}

impl Notify {
    pub fn new() -> Self {
        Self {
            core: Rc::new(RefCell::new(NotifyCore {
                waiters: VecDeque::new(),
                permit: None,
            })),
        }
    }

    pub fn wait(&self) {
        let mut core = self.core.borrow_mut();
        if let Some(()) = core.permit.take() {
            return;
        }
        let rt = runtime();
        core.waiters.push_back(rt.cur_task());
        drop(core);
        rt.yield_to_base(Packet::<()>::block_on(crate::task::BlockCause::Notify));
    }

    pub fn notify_one(&self) {
        let mut core = self.core.borrow_mut();
        if let Some(id) = core.waiters.pop_front() {
            wake_task(id);
        } else {
            core.permit = Some(());
        }
    }

    pub fn notify_last(&self) {
        let mut core = self.core.borrow_mut();
        if let Some(id) = core.waiters.pop_back() {
            wake_task(id);
        } else {
            core.permit = Some(());
        }
    }

    pub fn notify_waiters(&self) {
        let mut core = self.core.borrow_mut();
        while let Some(id) = core.waiters.pop_front() {
            wake_task(id);
        }
    }
}