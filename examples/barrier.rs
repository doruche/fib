use fib::{sync::Barrier, task};


#[fib::main]
fn main() {
    let barrier = Barrier::new(4);
    let mut handles = vec![];
    for i in 0..3 {
        let barrier_clone = barrier.clone();
        let handle = task::spawn(move || {
            println!("Task {} is waiting at the barrier", i + 1);
            let wait_result = barrier_clone.wait();
            println!("Task {} has passed the barrier", i + 1);
            wait_result
        });
        handles.push(handle);
    }
    // spawn some more tasks to test the barrier
    let mut other_handles = vec![];
    for i in 0..5 {
        other_handles.push(task::spawn(move || {
            println!("Task {} is doing some work", i + 4);
            task::yield_now();
            println!("Task {} has finished work", i + 4);
        }));
    }

    for handle in other_handles {
        handle.join();
    }

    let result = barrier.wait();
    for handle in handles {
        let result = handle.join();
        if result.is_leader() {
            println!("Leader task has passed the barrier");
        } else {
            println!("Non-leader task has passed the barrier");
        }
    }
    if result.is_leader() {
        println!("The last task is the leader and has passed the barrier");
    }
}