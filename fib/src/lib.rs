//! Fib is a conceptual implementation of the M:1, stackful, fiber-based and cooperative concurrency model in Rust.

#![allow(unused)]

mod utils;
mod config;
pub mod task;
pub mod sync;
pub mod runtime;

pub use fib_macros::main;