extern crate faster_rs;

use faster_rs::FasterKv;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread;

#[test]
fn multi_threaded_test() {
    let store = Arc::new(FasterKv::default());
    let ops = 1 << 15;

    let initial_value: u64 = 100;
    let modification: u64 = 30;
    store.start_session();

    for key in 0..ops {
        store.upsert(&(key as u64), &initial_value, key);
    }

    let num_threads = 16;
    let mut threads = vec![];
    for _ in 0..num_threads {
        let store = Arc::clone(&store);
        threads.push(thread::spawn(move || {
            // Register FASTER thread
            let _session = store.start_session();

            for key in 0..ops {
                store.rmw(&(key as u64), &modification, key);
            }

            // Make sure everything is completed
            store.complete_pending(true);

            // Unregister Thread
            store.stop_session();
        }))
    }

    for t in threads {
        t.join().unwrap();
    }

    for key in 0..ops {
        let expected_value = initial_value + (modification * num_threads);
        let (_res, recv): (u8, Receiver<u64>) = store.read(&key, ops + key);
        assert_eq!(recv.recv().unwrap(), expected_value);
    }
    store.complete_pending(true);
    store.stop_session();
}
