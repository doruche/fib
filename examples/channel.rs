use fib::{sync, task};

#[fib::main]
fn main() {
    let (tx, rx) = sync::mpsc::sync_channel(1);

    for i in 1..5 {
        let tx_clone = tx.clone();
        let _ = task::spawn(move || {
            tx_clone.send(format!("Message {}", i)).unwrap();
            task::yield_now();
            tx_clone.send(format!("Message {} after yield", i)).unwrap();
        });
    }
    drop(tx);
    
    loop {
        let res = rx.recv();
        match res {
            Ok(data) => println!("Received: {}", data),
            Err(e) => {
                println!("Error: {:?}", e);
                break;
            }
        }
    }
}