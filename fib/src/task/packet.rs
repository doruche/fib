//! Packet is for transferring data between tasks and the scheduler.

use crate::task::BlockCause;

pub(crate) enum Packet<R: 'static> {
    Yield,
    BlockOn(BlockCause),
    Result(Box<R>),
}

impl<R: 'static> Packet<R> {
    pub fn result(data: R) -> Box::<Self> {
        Box::new(Packet::Result(Box::new(data)))
    }

    pub fn _yield() -> Box::<Self> {
        Box::new(Packet::Yield)
    }

    pub fn block_on(cause: BlockCause) -> Box::<Self> {
        Box::new(Packet::BlockOn(cause))
    }

    pub unsafe fn raw_ptr(self: Box::<Self>) -> usize {
        unsafe {
            Box::into_raw(self) as usize
        }
    }
}