use fib::{sync::oneshot::{self, TryRecvError}, task};


#[fib::main]
fn main() {
    let (tx, mut rx) = oneshot::channel();
    let handle = task::spawn(move || {
        println!("Sending message from task");
        tx.send("Hello from task").unwrap();
    });
    println!("Waiting for message in main");
    
    let res = rx.try_recv();
    match res {
        Err(TryRecvError::Empty) => {
            println!("Channel is empty, waiting for message");
            handle.join();
        },
        _ => unreachable!(),
    }
    let msg = rx.blocking_recv().unwrap();

    println!("Received message: {}", msg);
}