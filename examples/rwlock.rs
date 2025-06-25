use std::rc::Rc;

use fib::{sync::RwLock, task};

#[fib::main]
fn main() {
    let counter = Rc::new(RwLock::new(0));
    let mut handles = vec![];
    for i in 0..10 {
        let counter_clone = Rc::clone(&counter);
        let handle = task::spawn(move || {
            for _ in 0..10 {
                let mut guard = counter_clone.write();
                *guard += 1;
                println!("Task {} incremented counter to {}", i, *guard);
                task::yield_now();
                println!("Task {} finished incrementing", i);
                drop(guard);
            }
        });
        handles.push(handle);

        let counter_clone = Rc::clone(&counter);
        let handle = task::spawn(move || {
            for _ in 0..10 {
                let reader = counter_clone.read();
                println!("Task {} read counter value: {}", i + 10, *reader);
                assert!(*reader >= 0);
                task::yield_now();
                println!("Task {} finished reading", i + 10);
                drop(reader);
            }
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join();
    }

    let final_value = counter.read();
    println!("Final value: {}", *final_value);
}