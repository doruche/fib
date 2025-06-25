//! Synchronization primitives

pub(crate) mod mutex;
pub(crate) mod notify;
pub mod mpsc;
pub mod oneshot;

pub use mutex::{
    Mutex,
    MutexGuard,
};

pub use notify::Notify;