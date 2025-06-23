//! Fib is a library for building asynchronous applications in Rust.
//! It provide a theoretical implementation of stackful, fiber-based, m:1 concurrency model.

#![allow(unused)]

mod utils;
mod config;
pub mod task;
pub mod sync;
pub mod runtime;