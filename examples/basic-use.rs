use std::rc::Rc;

use fib::task;
use fib::sync;

#[fib::main]
fn main() {
    let counter = Rc::new(sync::Mutex::new(0));
    let mut handles = vec![];
    for i in 0..10 {
        let c = counter.clone();
        let handle = task::spawn(move || {
            for _ in 0..5 {
                let mut guard = c.lock();
                *guard += 1;
                println!("Task {} incremented counter to {}", i, *guard);
                task::yield_now();
                *guard += 1;
                println!("Task {} incremented counter to {}", i, *guard);
            }
            format!("Ending from task {}", i)
        });
        handles.push(handle);
    }
    for handle in handles {
        let result = handle.join();
        println!("{}", result);
    }
}