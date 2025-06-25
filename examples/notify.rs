use fib::{sync::Notify, task};


#[fib::main]
fn main() {
    let notify = Notify::new();

    let handle = task::spawn({
        let notify = notify.clone();
        move || {
            println!("Waiting for notification...");
            notify.wait();
            println!("Received notification!");
        }
    });

    task::yield_now();

    println!("Sending notification...");
    notify.notify_one();

    handle.join();
}