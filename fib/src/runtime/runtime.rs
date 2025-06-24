use std::{collections::{HashMap, VecDeque}, rc::Rc};

use context::{Context, Transfer};

use crate::task::{packet::Packet, task::{AnyTask, JoinHandle, Task, TaskState}, BlockCause};


/// SAFETY  We have multiple mutable references to the runtime at the same time,
/// which is caused by the fact that we have to keep a mutable reference to do a yield.
/// So we should ensure that each access to the runtime is synchronized,
/// i.e. updating the current context, yielding, etc.
pub struct Runtime {
    base_cx: Option<Context>,
    pub(crate) cxs: HashMap<usize, Context>,
    pub(crate) running_tasks: VecDeque<Box<dyn AnyTask>>,
    pub(crate) blocking_tasks: HashMap<usize, Box<dyn AnyTask>>,
    cur_task: usize,
    next_id: usize,
}

impl Runtime {
    pub(crate) fn new() -> Self {
        Self {
            base_cx: None,
            cxs: HashMap::new(),
            running_tasks: VecDeque::new(),
            blocking_tasks: HashMap::new(),
            cur_task: usize::MAX,
            next_id: 0,
        }
    }

    pub(crate) fn cur_task(&self) -> usize {
        self.cur_task
    }

    pub(crate) fn next_id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id - 1
    }

    /// Yield to the scheduler.
    /// Called by tasks to relinquish control.
    pub(crate) fn yield_to_base<R: 'static>(&mut self, packet: Box<Packet<R>>) {
        let base_cx = self.base_cx.take().expect("No base context set");
        let to_base = Transfer::new(base_cx, 0);
        let packet = unsafe { packet.raw_ptr() };
        let from_base = unsafe { to_base.context.resume(packet) };
        self.base_cx = Some(from_base.context);
    }

    pub(crate) fn wake_task(&mut self, id: usize) {
        let mut task = self.blocking_tasks.remove(&id)
            .expect("No task found for the given id");
        task.trans_state(TaskState::Ready);
        self.running_tasks.push_back(task);
    }

    pub(crate) fn spawn<F, R>(&mut self, future: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + 'static,
        R: 'static,
    {
        let id = self.next_id();
        let (task, init_cx) = Task::new(id, future);
        let result = task.result.clone();
        self.running_tasks.push_back(Box::new(task));
        self.cxs.insert(id, init_cx);
        
        JoinHandle { id, result }
    }


    pub fn block_on<F, R>(&mut self, future: F) -> R
    where 
        F: FnOnce() -> R + 'static,
        R: 'static,
    {
        let root_handle = self.spawn(future);

        loop {
            match self.running_tasks.pop_front() {
                Some(mut task) => {
                    self.cur_task = task.id();
                    task.resume();
                    match task.state() {
                        TaskState::Finished => {},
                        TaskState::Ready => self.running_tasks.push_back(task),
                        TaskState::BlockOn(cause) => match cause {
                            BlockCause::Lock | BlockCause::Channel => {
                                assert!(self.blocking_tasks.insert(task.id(), task).is_none())
                            }
                        },
                        TaskState::Running => unreachable!(),
                    }
                },
                None => {
                    // TODO: Handle blocking I/O tasks
                    if !self.blocking_tasks.is_empty() {
                        continue;
                    }
                    break;
                },
            }
        }

        Rc::into_inner(root_handle.result)
            .unwrap()
            .into_inner()
            .unwrap()
    }

    pub(crate) fn get_cur_cx(&mut self) -> Option<context::Context> {
        self.cxs.remove(&self.cur_task)
    }

    pub(crate) fn set_cur_cx(&mut self, cx: context::Context) {
        assert!(self.cxs.insert(self.cur_task, cx).is_none());
    }

    pub(crate) fn set_base_cx(&mut self, cx: context::Context) {
        assert!(self.base_cx.is_none(), "Base context already set");
        self.base_cx = Some(cx);
    }

    pub(crate) fn get_base_cx(&mut self) -> Option<context::Context> {
        self.base_cx.take()
    }
}