extern crate faster_kvs;

use faster_kvs::FasterKv;

#[test]
fn session_check() {
    let table_size: u64  = 1 << 14;
    let log_size: u64 = 17179869184;
    if let Ok(store) = FasterKv::new(table_size, log_size, String::from("test_session")) {
        let value: u64 = 100;

        let session_one_guid = store.start_session();
        assert!(session_one_guid.len() == 36);

        for key in 0..1000 {
            store.upsert(key as u64, value);
        }

        store.complete_pending(true);
        store.stop_session();


        let session_two_guid = store.start_session();

        for key in 0..1000 {
            store.upsert(key as u64, value);
        }

        store.complete_pending(true);
        store.stop_session();


        let r = store.recover(session_two_guid.clone(), session_two_guid.clone()).unwrap();
        assert_eq!(r.version, 0);
        //assert_eq!(r.status, 0);
        //assert_eq!(r.session_ids.len(), 2);

        match store.clean_storage() {
            Ok(()) => assert!(true),
            Err(_err) => assert!(false)
        }
    } else {
        assert!(false);
    }
}
