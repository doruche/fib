use std::rc::Rc;

use fib::{sync::Semaphore, task};


#[fib::main]
fn main() {
    let sem = Rc::new(Semaphore::new(10));
    let mut handles = vec![];

    for _ in 0..50 {
        let sem_clone = sem.clone();
        handles.push(task::spawn(move || {
            loop {
                match sem_clone.try_acquire() {
                    Ok(_permit) => {
                        println!("Acquired permit, available: {}", sem_clone.available_permits());
                        task::yield_now();
                        break;
                    }
                    Err(e) => {
                        println!("Failed to acquire permit: {:?}", e);
                        task::yield_now(); // Yield to allow other tasks to run
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.join();
    }

    println!("All tasks completed.");
}