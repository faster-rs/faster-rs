extern crate faster_kvs;
extern crate serde_derive;

use faster_kvs::{FasterKv, FasterValue,status};
use serde_derive::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;

// Note: Debug annotation is just for printing later
#[derive(Serialize, Deserialize, Debug)]
struct MyValue {
    foo: String,
    bar: String,
}

impl FasterValue<'_, MyValue> for MyValue {
    fn rmw(&self, modification: MyValue) -> MyValue {
        unimplemented!()
    }
}

fn main() {
    const TABLE_SIZE: u64  = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;

    // Create a Key-Value Store
    if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("example_custom_values_storage")) {
        let key: u64 = 1;
        let value = MyValue { foo: String::from("Hello"), bar: String::from("World") };

        // Upsert
        let upsert = store.upsert(key, &value);
        assert!(upsert == status::OK || upsert == status::PENDING);

        assert!(store.size() > 0);

        // Note: need to provide type annotation for the Receiver
        let (read, recv): (u8, Receiver<MyValue>) = store.read(key);
        assert!(read == status::OK || read == status::PENDING);
        let val = recv.recv().unwrap();
        println!("Key: {}, Value: {:?}", key, val);

        // Clear used storage
        match store.clean_storage() {
            Ok(()) => {},
            Err(_err) => panic!("Unable to clear FASTER directory"),
        }
    } else {
        panic!("Unable to create FASTER directory");
    }
}
