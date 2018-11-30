extern crate faster_kvs;

use faster_kvs::*;

fn main() {
    const TABLE_SIZE: u64  = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;
    
    if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("storage")) {
        let key: u64 = 1;
        let value: u64 = 1000;

        println!("Upserting with key: {}, value: {}", key, value);
        store.upsert(key, value);

        let res = store.read(key);
        assert!(res == status::OK);

        let res = store.read(2 as u64);
        assert!(res == status::NOT_FOUND);

        let rmw_val: u64 = 5;
        println!("Read-Modify-Write with key: {}, value: {}", key, rmw_val);
        let rmw = store.rmw(key, rmw_val);
        assert!(rmw == status::OK);

        println!("Store size: {}", store.size());


        match store.checkpoint() {
            Some(c) => {
                let rec = store.recover(c.token.clone(), c.token.clone()).unwrap();
                println!("{}", rec.status);
                println!("{}", rec.version);
            },
            None =>  println!("checkpointing failed")
        }

        match store.clean_storage() {
            Ok(()) => println!("{}", "Cleaned storage"),
            Err(err) => panic!(err)
        }

    } else {
        println!("{}", "Failed setting up FasterKv");
    }
}
