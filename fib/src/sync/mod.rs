//! Synchronization primitives

pub(crate) mod mutex;
pub mod mpsc;
pub mod oneshot;

pub use mutex::{
    Mutex,
    MutexGuard,
};