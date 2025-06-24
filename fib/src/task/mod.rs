//! Task management module

use crate::{runtime::runtime, task::{packet::Packet, task::JoinHandle}};

pub(crate) mod task;
pub(crate) mod packet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockCause {
    Lock,
    Channel,
}

pub fn yield_now() {
    let rt = runtime();
    rt.yield_to_base(Packet::<()>::_yield());
}

pub fn wait(id: usize) {
    let mut rt = runtime();
    while rt.cxs.contains_key(&id) {
        yield_now();
    }
}

pub fn spawn<F, R>(future: F) -> JoinHandle<R>
where
    F: FnOnce() -> R + 'static,
    R: 'static,
{
    let mut rt = runtime();
    rt.spawn(future)
}