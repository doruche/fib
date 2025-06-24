//! Synchronization primitives

pub(crate) mod mutex;
pub mod mpsc;

pub use mutex::{
    Mutex,
    MutexGuard,
};