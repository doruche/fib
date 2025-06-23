//! Macros for use with fib.

#![allow(unused)]

extern crate proc_macro;

use proc_macro::TokenStream;

mod entry;

#[proc_macro_attribute]
pub fn main(_args: TokenStream, item: TokenStream) -> TokenStream {
    entry::main_impl(item)
}