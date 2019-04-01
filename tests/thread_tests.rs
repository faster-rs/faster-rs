extern crate faster_kvs;
extern crate tempfile;

use tempfile::TempDir;
use faster_kvs::FasterKv;
use std::thread;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

#[test]
fn multi_threaded_test() {
    let table_size: u64  = 1 << 14;
    let log_size: u64 = 17179869184;
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = Arc::new(FasterKv::new(table_size, log_size, dir_path).unwrap());
    let ops = 1000;

    let initial_value: u64 = 100;
    let modification: u64 = 30;

    for key in 0..ops {
        store.upsert(key as u64, &initial_value);
    }

    let num_threads = 4;
    let mut threads = vec![]; 
    for _ in 0..num_threads {
        let store = Arc::clone(&store);
        threads.push(thread::spawn(move || {
            // Register FASTER thread
            let _session = store.start_session();

            for key in 0..ops {
                store.rmw(key as u64, &modification);
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
        let (_res, recv): (u8, Receiver<u64>) = store.read(key as u64);
        assert_eq!(recv.recv().unwrap(), expected_value);
    }
}
