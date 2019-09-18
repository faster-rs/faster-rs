extern crate faster_rs;
extern crate serde_derive;

use faster_rs::{status, FasterKv};
use serde_derive::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;

// Note: Debug annotation is just for printing later
#[derive(Serialize, Deserialize, Debug)]
struct MyValue {
    foo: String,
    bar: String,
}

fn main() {
    // Create a Key-Value Store
    let store = FasterKv::default();
    let key: u64 = 1;
    let value = MyValue {
        foo: String::from("Hello"),
        bar: String::from("World"),
    };

    // Upsert
    let upsert = store.upsert(&key, &value, 1);
    assert!(upsert == status::OK || upsert == status::PENDING);

    assert!(store.size() > 0);

    // Note: need to provide type annotation for the Receiver
    let (read, recv): (u8, Receiver<MyValue>) = store.read(&key, 1);
    assert!(read == status::OK || read == status::PENDING);
    let val = recv.recv().unwrap();
    println!("Key: {}, Value: {:?}", key, val);

    // Clear used storage
    match store.clean_storage() {
        Ok(()) => {}
        Err(_err) => panic!("Unable to clear FASTER directory"),
    }
}
