use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use crate::{runtime::runtime, task::packet::Packet};

struct BarrierCore {
    threshold: usize,
    count: usize,
    // cur_generation: usize,
    waiters: VecDeque<usize>,
}

#[derive(Clone)]
pub struct Barrier {
    core: Rc<RefCell<BarrierCore>>,
    // generation: usize,
}

#[derive(Debug)]
pub struct BarrierWaitResult {
    is_leader: bool,
}

impl Barrier {
    pub fn new(n: usize) -> Self {
        assert!(n > 0, "Barrier threshold must be greater than zero");
        Self {
            core: Rc::new(RefCell::new(BarrierCore {
                threshold: n,
                count: n,
                waiters: VecDeque::new(),
            })),
        }
    }

    pub fn wait(&self) -> BarrierWaitResult {
        let mut core = self.core.borrow_mut();
        core.count -= 1;
        if core.count == 0 {
            core.count = core.threshold;
            let rt = runtime();
            while let Some(waiter) = core.waiters.pop_front() {
                rt.wake_task(waiter);
            }
            BarrierWaitResult { is_leader: true }
        } else {
            let rt = runtime();
            core.waiters.push_back(rt.cur_task());
            drop(core);
            rt.yield_to_base(Packet::<()>::block_on(crate::task::BlockCause::Barrier));            
            BarrierWaitResult { is_leader: false }
        }
    }
}

impl BarrierWaitResult {
    pub fn is_leader(&self) -> bool {
        self.is_leader
    }
}