extern crate faster_rs;

use faster_rs::FasterKv;
use std::sync::Arc;
use std::thread;

fn add_numbers(val: &[u8], modification: &[u8]) -> Vec<u8> {
    let mut result = 0;
    result += bincode::deserialize::<u64>(val).unwrap();
    result += bincode::deserialize::<u64>(modification).unwrap();
    bincode::serialize(&result).unwrap()
}

#[test]
fn multi_threaded_test() {
    let store = Arc::new(FasterKv::default());
    let ops = 1 << 15;

    let val: u64 = 100;
    let modif: u64 = 30;

    let initial_value = bincode::serialize(&val).unwrap();
    store.start_session();

    for key in 0..ops {
        store.upsert(&bincode::serialize(&key).unwrap(), &initial_value, key);
    }

    let num_threads = 16;
    let mut threads = vec![];
    for _ in 0..num_threads {
        let store = Arc::clone(&store);
        threads.push(thread::spawn(move || {
            // Register FASTER thread
            let _session = store.start_session();
            let modification = bincode::serialize(&modif).unwrap();

            for key in 0..ops {
                store.rmw(
                    &bincode::serialize(&key).unwrap(),
                    &modification,
                    add_numbers,
                    key,
                );
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
        let expected_value = val + (modif * num_threads);
        let (_res, recv) = store.read(&bincode::serialize(&key).unwrap(), ops + key);
        assert_eq!(
            bincode::deserialize::<u64>(&recv.recv().unwrap()).unwrap(),
            expected_value
        );
    }
    store.complete_pending(true);
    store.stop_session();
}
