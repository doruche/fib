use std::{cell::RefCell, collections::VecDeque, fmt::Debug, rc::Rc};

use crate::{runtime::{cur_task, runtime, wake_task}, sync::Mutex, task::{packet::Packet, BlockCause}};

struct Channel<T> {
    buffer: VecDeque<T>,
    receiver_waiter: Option<usize>,
    closed: bool,
}

pub fn channel<T: 'static>() -> (Sender<T>, Receiver<T>) {
    let channel = Rc::new(RefCell::new(Channel {
        buffer: VecDeque::<T>::new(),
        receiver_waiter: None,
        closed: false,
    }));
    
    let sender = Sender {
        channel: channel.clone(),
        sender_count: Rc::new(()),
    };
    
    let receiver = Receiver {
        channel,
    };
    
    (sender, receiver)
}

struct SyncChannel<T> {
    buffer: VecDeque<T>,
    capacity: usize,
    sender_waiters: VecDeque<usize>,
    receiver_waiter: Option<usize>,
    closed: bool,
}

#[derive(Debug)]
pub enum SendError {
    Disconnected,
}

#[derive(Debug)]
pub enum RecvError {
    Disconnected,
}

/// Non-blocking primitives
trait ChannelTrait<T> {
    fn send(&mut self, item: T) -> Result<Option<T>, SendError>;
    fn recv(&mut self) -> Result<Option<T>, RecvError>;

    fn add_recv_waiter(&mut self, id: usize);
    fn add_sender_waiter(&mut self, id: usize);
    fn close(&mut self);
    fn is_closed(&self) -> bool;
}

#[derive(Clone)]
pub struct Sender<T> {
    channel: Rc<RefCell<dyn ChannelTrait<T>>>,
    sender_count: Rc<()>,
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if Rc::strong_count(&self.sender_count) == 1 {
            let mut channel = self.channel.borrow_mut();
            channel.close();
        }
    }
}

pub struct Receiver<T> {
    channel: Rc<RefCell<dyn ChannelTrait<T>>>,
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut channel = self.channel.borrow_mut();
        channel.close();
    }
}

impl<T> Sender<T> {
    pub fn send(&self, mut item: T) -> Result<(), SendError> {
        loop {
            let mut channel = self.channel.borrow_mut();
            let res = channel.send(item);
            match res {
                // WouldBlock
                Ok(Some(item_back)) => {
                    item = item_back;
                    let rt = runtime();
                    channel.add_sender_waiter(rt.cur_task());
                    drop(channel);
                    rt.yield_to_base(Packet::<()>::block_on(BlockCause::Channel));
                },
                Ok(None) => return Ok(()),
                Err(SendError::Disconnected) => return Err(SendError::Disconnected),
            }
        }
    }
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> Result<T, RecvError> {
        loop {
            let mut channel = self.channel.borrow_mut();
            let res = channel.recv();
            match res {
                // WouldBlock
                Ok(None) => {
                    let rt = runtime();
                    channel.add_recv_waiter(rt.cur_task());
                    drop(channel);
                    rt.yield_to_base(Packet::<()>::block_on(BlockCause::Channel));
                },
                Ok(Some(item)) => return Ok(item),
                Err(RecvError::Disconnected) => return Err(RecvError::Disconnected),
            }
        }
    }
}

impl<T> ChannelTrait<T> for Channel<T> {
    fn send(&mut self, item: T) -> Result<Option<T>, SendError> {
        if self.closed {
            return Err(SendError::Disconnected);
        }
        self.buffer.push_back(item);
 
        if let Some(waiter_id) = self.receiver_waiter.take() {
            wake_task(waiter_id);
        }
 
        Ok(None)
    }

    fn recv(&mut self) -> Result<Option<T>, RecvError> {
        assert!(self.receiver_waiter.is_none());
        if self.buffer.is_empty() && self.closed {
            return Err(RecvError::Disconnected);
        }
        if self.buffer.is_empty() {
            return Ok(None);
        }

        let item = self.buffer.pop_front().unwrap();

        Ok(Some(item))
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
    
    fn close(&mut self) {
        self.closed = true;
        if let Some(recv_waiter) = self.receiver_waiter.take() {
            wake_task(recv_waiter);
        }
    }

    fn add_recv_waiter(&mut self, id: usize) {
        assert!(self.receiver_waiter.is_none());
        self.receiver_waiter = Some(id);
    }
    
    fn add_sender_waiter(&mut self, id: usize) {
        // Asynchronous channel has no sender waiters.
        unimplemented!()
    }
}