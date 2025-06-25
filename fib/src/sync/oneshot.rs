use std::{cell::{OnceCell, RefCell}, rc::Rc};

use crate::{runtime::{runtime, wake_task}, task::{packet::Packet, BlockCause}};

pub fn channel<T: 'static>() -> (Sender<T>, Receiver<T>) {
    let channel = Rc::new(RefCell::new(Channel {
        item: OnceCell::new(),
        receiver_waiter: None,
        closed: false,
    }));
    (
        Sender {
            channel: channel.clone(),
        },
        Receiver {
            channel,
        },
    )
}

struct Channel<T> {
    item: OnceCell<T>,
    receiver_waiter: Option<usize>,
    closed: bool,
}

pub struct Sender<T> {
    channel: Rc<RefCell<Channel<T>>>,
}

pub struct Receiver<T> {
    channel: Rc<RefCell<Channel<T>>>,
}

#[derive(Debug)]
pub enum RecvError {
    Closed,
}

#[derive(Debug)]
pub enum TryRecvError {
    Empty,
    Closed,
}


impl<T> Sender<T> {
    pub fn send(self, item: T) -> Result<(), T> {
        let mut channel = self.channel.borrow_mut();
        if channel.closed {
            return Err(item);
        }
        if let Err(item) = channel.item.set(item) {
            return Err(item);
        }
        if let Some(receiver) = channel.receiver_waiter.take() {
            wake_task(receiver);
        }
        Ok(())
    }

    pub fn is_closed(&self) -> bool {
        self.channel.borrow().closed
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut channel = self.channel.borrow_mut();
        channel.closed = true;
        if let Some(receiver) = channel.receiver_waiter.take() {
            wake_task(receiver);
        }
    }
}

impl<T> Receiver<T> {
    pub fn is_empty(&self) -> bool {
        self.channel.borrow().item.get().is_none()
    }

    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        let mut channel = self.channel.borrow_mut();
        
        if let Some(item) = channel.item.take() {
            Ok(item)
        } else {
            if channel.closed {
                Err(TryRecvError::Closed)
            } else {
                Err(TryRecvError::Empty)
            }
        }
    }

    pub fn blocking_recv(self) -> Result<T, RecvError> {
        let mut channel = self.channel.borrow_mut();

        if let Some(item) = channel.item.take() {
            return Ok(item);
        }
        if channel.closed {
            return Err(RecvError::Closed);
        }

        let rt = runtime();
        channel.receiver_waiter = Some(rt.cur_task());
        drop(channel);
        rt.yield_to_base(Packet::<()>::block_on(BlockCause::Channel));
        channel = self.channel.borrow_mut();

        if let Some(item) = channel.item.take() {
            Ok(item)
        } else {
            Err(RecvError::Closed)
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.channel.borrow_mut().closed = true;
    }
}