# Fib
A conceptual implementation of the __M:1, stackful, fiber-based and cooperative__ concurrency model in Rust.
## Overview
- What Is A Fiber?
  - A fiber is a lightweight thread of execution that can be paused and resumed, allowing for cooperative multitasking.
  - The exact definition of a fiber can vary, but it generally refers to a user-level thread that is managed by a runtime rather than the operating system. Different from the `async`/`await` model, fibers are stackful, meaning they maintain their own stack and can yield control at any point in their execution. In `fib`, we allocate a fixed-size stack (currently 32 KiB, see `fib/src/config.rs`) with a guard page to prevent stack overflows for each fiber.
- Pros & Cons of Fiber?
  - Pros
    - Context switching is quite faster than OS threads.
    - More memory-efficient than OS threads, as fibers typically require smaller stack allocations compared to the default large stacks of OS threads.
    - Users take full control of the scheduling, allowing for more predictable performance.
  - Cons
    - Requires explicit yielding, which can lead to less responsive applications if not managed properly.   (__NOTICE__ For stackful coroutines, __Fiber__ generally means cooperative multitasking while the __Green Thread__ stands for preemptive multitasking.)
    - No easy way to manage the need to grow the stack if a fiber runs out of stack space.
    - Context switching is still required, which incur more overhead compared to the async/await model's compile-time optimizations for state machines.
  - M:1 Concurrency Model?
    - Basically, it means that multiple fibers are scheduled on __one single kernel thread__, which have apparent advantages and disadvantages.
    - Advantages:
      - Incredibly lightweight.
      - No need to deal with the complexities of OS threads, such as thread pools, synchronization, and so on.
    - Disadvantages:
      - No true parallelism, as all fibers run on a single thread.
      - Performance is divided by the single thread, which can lead to bottlenecks in CPU-bound tasks.
    - For I/O bound tasks, we can use non-blocking I/O to avoid blocking the fiber (epoll() in Linux, kqueue() in BSD, IOCP in Windows, etc.) to achieve better performance. However for CPU-bound tasks, we can not get true parallelism, and this is a most common problem with M:1 concurrency model. 
## Maybe The Biggest Pity...
__TLDR; We lose the assistance from Rust's type system and compile-time checks__ when using fibers, as the _Fearless Concurrency_ model is based on `Send`/`Sync` traits, which is to deal with OS threads, while fibers are user-level threads. <br/>
So we come to an awkward situation: users of fibers have to manually ensure that the code is safe to run in a fiber, which can lead to runtime errors if not done correctly.<br/> Example: 
```rust
use std::cell::RefCell;
use std::rc::Rc;
use fib::task;

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
```
The safety of `RefCell` is based on the assumption that no more than one mutable reference exists at a time, which can easily be violated in the fiber context. This is because fibers can yield and resume at any point, leading to potential data races and other concurrency issues.
## How To Synchronize Fibers?
Answer: Use `fib::sync` module, which provides a set of synchronization primitives that are safe to use in fibers. These primitives are designed to work with the fiber model and provide a way to safely share data between fibers without violating the safety guarantees of Rust. <br/>
Should notice that the `std::sync` is useless in fibers, as it is designed for OS threads and does not work with the fiber model. So use `fib::sync::Mutex` instead of `std::sync::Mutex` and so on. The latter can easily cause problems - assume that you acquire a std mutex and yield, then another fiber tries to acquire the same mutex, thus leading to a deadlock.

---

__Currently Supported:__
  - Mutex
  - Channel
  - Sync Channel
  - OneShot
## Example
```rust
// examples/basic-use.rs

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
```
More examples can be found in `examples` directory.
## License
This project is licensed under the [MIT License](LICENSE).