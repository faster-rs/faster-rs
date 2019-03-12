extern crate faster_kvs;

use faster_kvs::FasterKv;

#[test]
fn single_checkpoint() {
    let table_size: u64  = 1 << 14;
    let log_size: u64 = 17179869184;
    if let Ok(store) = FasterKv::new(table_size, log_size, String::from("test_checkpoint")) {
        let value: u64 = 100;

        for key in 0..1000 {
            store.upsert(key as u64, &value);
        }

        let checkpoint = store.checkpoint().unwrap();
        assert_eq!(checkpoint.checked, true);
        assert_eq!(checkpoint.token.len(), 37-1); // -1 \0

        match store.clean_storage() {
            Ok(()) => assert!(true),
            Err(_err) => assert!(false)
        }
    } else {
        assert!(false);
    }
}


#[test]
fn concurrent_checkpoints() {
    //TODO
}
