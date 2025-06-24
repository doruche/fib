//! Task management module
//! Task is our representation of a fiber.

use std::{cell::OnceCell, rc::Rc};

use context::{stack::ProtectedFixedSizeStack, Transfer};

use crate::runtime::runtime;
use crate::task::packet::Packet;
use crate::task::wait;
use crate::{runtime::task_entry, task::BlockCause};
use crate::config::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TaskState {
    Ready,
    Running,
    BlockOn(BlockCause),
    Finished,
}

pub(crate) struct Task<R: 'static> {
    pub(crate) id: usize,
    pub(crate) stack: ProtectedFixedSizeStack,
    pub(crate) state: TaskState,
    pub(crate) result: Rc<OnceCell<R>>,
}

impl<R: 'static> Task<R> {
    pub(crate) fn new<F>(id: usize, future: F) -> (Self, context::Context)
    where
        F: FnOnce() -> R + 'static,
    {
        let stack = ProtectedFixedSizeStack::new(STACK_SIZE).unwrap();
        let cx = unsafe { context::Context::new(&stack, task_entry::<R>) };
        
        // Set up initial state.
        let closure: Box<dyn FnOnce() -> R + 'static> = Box::new(future);
        let closure_ptr = unsafe { Box::into_raw(Box::new(closure)) } as usize;
        let mut to_task = Transfer::new(cx, 0);
        to_task = unsafe { to_task.context.resume(closure_ptr) };
        assert_eq!(to_task.data, 42);

        (Self {
            id,
            stack,
            state: TaskState::Ready,
            result: Rc::new(OnceCell::new()),
        }, to_task.context)
    }
}

pub(crate) trait AnyTask {
    fn id(&self) -> usize;
    fn resume(&mut self);
    fn trans_state(&mut self, new_state: TaskState);
    fn state(&self) -> TaskState;
}

impl<R: 'static> AnyTask for Task<R> {
    fn id(&self) -> usize {
        self.id
    }

    fn resume(&mut self) {
        let rt = runtime();
        // Weird, tighly coupled, ugly. But for simplicity we keep it like this.
        if let Some(cx) = rt.get_cur_cx() {
            match self.state {
                TaskState::Ready => {
                    self.state = TaskState::Running;
                    
                    let mut to_task = Transfer::new(cx, 0);
                    let mut from_task = unsafe { to_task.context.resume(0) };
                    rt.set_cur_cx(from_task.context);
                    
                    let packet = unsafe {
                        let packet_ptr = from_task.data as *mut Packet<R>;
                        *Box::from_raw(packet_ptr)
                    };
                    match packet {
                        Packet::Result(result) => {
                            assert!(self.result.set(*result).is_ok());
                            // rt.cxs.remove(&self.id);
                            rt.get_cur_cx().unwrap();
                            self.state = TaskState::Finished;
                        },
                        Packet::Yield =>  {
                            self.state = TaskState::Ready;
                        },
                        Packet::BlockOn(cause) => {
                            self.state = TaskState::BlockOn(cause);
                        }
                    }
                },
                TaskState::BlockOn(ref cause) => match cause {
                    _ => unreachable!(),
                },
                TaskState::Finished|TaskState::Running => unreachable!(),
            }
        } else {
            panic!("No context for task {}", self.id);
        }
    }

    fn state(&self) -> TaskState {
        self.state
    }

    fn trans_state(&mut self, new_state: TaskState) {
        self.state = new_state;
    }
}

pub struct JoinHandle<R: 'static> {
    pub(crate) id: usize,
    pub(crate) result: Rc<OnceCell<R>>,
}

impl<R: 'static> JoinHandle<R> {
    pub fn is_finished(&self) -> bool {
        self.result.get().is_some()
    }

    pub fn join(self) -> R {
        wait(self.id);
        Rc::into_inner(self.result)
            .unwrap()
            .into_inner()
            .unwrap()
    }
}
