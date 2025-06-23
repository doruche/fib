//! Synchronization primitives

pub(crate) mod mutex;

pub use mutex::{
    Mutex,
    MutexGuard,
};