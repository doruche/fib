// Runtime is the core of the Fib system, handling task management and execution.

pub(crate) mod runtime;

use context::Transfer;

use crate::{runtime::runtime::Runtime, task::packet::Packet, utils::STCell};

thread_local! {
    pub(crate) static RUNTIME: STCell<Runtime> = STCell::new(Runtime::new());
}

pub fn runtime() -> &'static mut Runtime {
    RUNTIME.with(|cell| unsafe {
        (*cell.inner.get()).as_mut().unwrap()
    })
}

pub(crate) extern "C" fn task_entry<R: 'static>(to_base: Transfer) -> ! {
    let closure_ptr = unsafe {
        let closure_ptr = to_base.data as *mut Box<dyn FnOnce() -> R + Send + 'static>;
        Box::from_raw(closure_ptr)
    };
    let closure =  *closure_ptr;

    let mut rt = runtime();

    rt.set_base_cx(unsafe { to_base.context.resume(42).context });

    let result = closure();

    let result = Packet::result(result);

    let base_cx = rt.get_base_cx().expect("Base context not set");
    let to_base = Transfer::new(base_cx, 0);
    unsafe { to_base.context.resume(result.raw_ptr()); }

    unreachable!()
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{sync::mutex::Mutex, task};

    use super::*;

    #[test]
    fn test_entry() {
        let mut rt = runtime();
        rt.block_on(test_mutex);
    }
    
    fn test_mutex() {
        let mut handles = vec![];
        let counter = Rc::new(Mutex::new(0));

        for i in 1..10 {
            let c = counter.clone();
            let handle = task::spawn(move || {
                let mut guard = c.lock();
                *guard += 1;
                println!("Task {} incremented counter to {}", i, *guard);
                task::yield_now();
                *guard += 1;
                println!("Task {} incremented counter to {}", i, *guard);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join();
        }

        println!("Final counter value: {}", *counter.lock());
    }
}