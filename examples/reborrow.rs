/// Following code was written intentionally to cause a reborrow error.

use std::{cell::RefCell, rc::Rc};
use fib::task;

#[fib::main]
fn main() {
    let cell = Rc::new(RefCell::new(0));
    let cell_clone = cell.clone();
    let _ = task::spawn(move || {
        // This will panic!
        let _a = cell.borrow_mut();
        unreachable!();
    });
    let _b = cell_clone.borrow_mut();
    task::yield_now();
    unreachable!();
}