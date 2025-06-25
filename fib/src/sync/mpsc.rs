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

/// __NOTE__ Zero capacity currently not supported.
pub fn sync_channel<T: 'static>(capacity: usize) -> (SyncSender<T>, Receiver<T>) {
    let channel = Rc::new(RefCell::new(SyncChannel {
        buffer: VecDeque::<T>::with_capacity(capacity),
        capacity,
        sender_waiters: VecDeque::new(),
        receiver_waiter: None,
        closed: false,
    }));

    let sender = SyncSender {
        inner: Sender {
            channel: channel.clone(),
            sender_count: Rc::new(()),
        },
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
pub enum SendError<T> {
    Disconnected(T),
}

#[derive(Debug)]
pub enum TrySendError<T> {
    Full(T),
    Disconnected(T),
}

#[derive(Debug)]
pub enum RecvError {
    Disconnected,
}

#[derive(Debug)]
pub enum TryRecvError {
    Empty,
    Disconnected,
}

/// Non-blocking primitives
trait ChannelTrait<T> {
    fn send(&mut self, item: T) -> Result<(), TrySendError<T>>;
    fn recv(&mut self) -> Result<T, TryRecvError>;

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

#[derive(Clone)]
pub struct SyncSender<T> {
    inner: Sender<T>,
}

impl<T> SyncSender<T> {
    pub fn send(&self, item: T) -> Result<(), SendError<T>> {
        self.inner.send(item)
    }

    pub fn try_send(&self, item: T) -> Result<(), TrySendError<T>> {
        let mut channel = self.inner.channel.borrow_mut();
        channel.send(item)
    }
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
    pub fn send(&self, mut item: T) -> Result<(), SendError<T>> {
        loop {
            let mut channel = self.channel.borrow_mut();
            let res = channel.send(item);
            match res {
                Ok(()) => return Ok(()),
                // WouldBlock
                Err(TrySendError::Full(item_back)) => {
                    item = item_back;
                    let rt = runtime();
                    channel.add_sender_waiter(rt.cur_task());
                    drop(channel);
                    rt.yield_to_base(Packet::<()>::block_on(BlockCause::Channel));
                },
                Err(TrySendError::Disconnected(item_back)) => return Err(SendError::Disconnected(item_back)),
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
                Ok(item) => return Ok(item),
                // WouldBlock
                Err(TryRecvError::Empty) => {
                    let rt = runtime();
                    channel.add_recv_waiter(rt.cur_task());
                    drop(channel);
                    rt.yield_to_base(Packet::<()>::block_on(BlockCause::Channel));
                },
                Err(TryRecvError::Disconnected) => return Err(RecvError::Disconnected),
            }
        }
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let mut channel = self.channel.borrow_mut();
        channel.recv()
    }
}

impl<T> ChannelTrait<T> for Channel<T> {
    fn send(&mut self, item: T) -> Result<(), TrySendError<T>> {
        if self.closed {
            return Err(TrySendError::Disconnected(item));
        }
        self.buffer.push_back(item);
 
        if let Some(waiter_id) = self.receiver_waiter.take() {
            wake_task(waiter_id);
        }
 
        Ok(())
    }

    fn recv(&mut self) -> Result<T, TryRecvError> {
        assert!(self.receiver_waiter.is_none());
        if self.buffer.is_empty() && self.closed {
            return Err(TryRecvError::Disconnected);
        }
        if self.buffer.is_empty() {
            return Err(TryRecvError::Empty);
        }

        let item = self.buffer.pop_front().unwrap();

        Ok(item)
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

impl<T> ChannelTrait<T> for SyncChannel<T> {
    fn send(&mut self, item: T) -> Result<(), TrySendError<T>> {
        if self.closed {
            return Err(TrySendError::Disconnected(item));
        }
        if self.buffer.len() >= self.capacity {
            return Err(TrySendError::Full(item));
        }
        self.buffer.push_back(item);
        if let Some(waiter_id) = self.receiver_waiter.take() {
            wake_task(waiter_id);
        }
        if let Some(sender_waiter) = self.sender_waiters.pop_front() {
            wake_task(sender_waiter);
        }
        Ok(())
    }

    fn recv(&mut self) -> Result<T, TryRecvError> {
        assert!(self.receiver_waiter.is_none());
        if self.buffer.is_empty() && self.closed {
            return Err(TryRecvError::Disconnected);
        }
        if self.buffer.is_empty() {
            return Err(TryRecvError::Empty);
        }
        let item = self.buffer.pop_front().unwrap();

        if let Some(sender_waiter) = self.sender_waiters.pop_front() {
            wake_task(sender_waiter);
        }

        Ok(item)
    }

    fn add_recv_waiter(&mut self, id: usize) {
        assert!(self.receiver_waiter.is_none());
        self.receiver_waiter = Some(id);
    }

    fn add_sender_waiter(&mut self, id: usize) {
        assert!(!self.sender_waiters.contains(&id));
        self.sender_waiters.push_back(id);
    }

    fn close(&mut self) {
        self.closed = true;
        if let Some(recv_waiter) = self.receiver_waiter.take() {
            wake_task(recv_waiter);
        }
        for sender_waiter in self.sender_waiters.drain(..) {
            wake_task(sender_waiter);
        }
    }

    fn is_closed(&self) -> bool {
        self.closed
    }
}