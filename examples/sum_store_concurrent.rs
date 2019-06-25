extern crate faster_rs;

use faster_rs::*;
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::Arc;

const TABLE_SIZE: u64 = 1 << 15;
const LOG_SIZE: u64 = 17179869184;
const NUM_OPS: u64 = 1 << 25;
const NUM_UNIQUE_KEYS: u64 = 1 << 22;
const REFRESH_INTERVAL: u64 = 1 << 8;
const COMPLETE_PENDING_INTERVAL: u64 = 1 << 12;
const CHECKPOINT_INTERVAL: u64 = 1 << 22;

const STORAGE_DIR: &str = "sum_store_concurrent_storage";

// More or less a copy of the multi-threaded sum_store populate/recover example from FASTER

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        let operation = &args[1].to_string();
        let num_threads = args[2]
            .parse()
            .expect("Must specify number of threads as an integer");

        if operation == "populate" {
            println!(
                "{}",
                "This may take a while, and make sure you have disk space"
            );
            populate(num_threads);
        } else if operation == "recover" {
            if args.len() > 3 {
                let token = &args[3];
                recover(token.to_string(), num_threads);
            } else {
                println!("Second argument required is checkpoint token to recover");
            }
        }
    } else {
        println!("Populate: args {}, {}", "1. populate", "2. #threads");
        println!(
            "Recover: args {}, {}, {}",
            "1. recover", "2. #threads", "3. checkpoint token"
        );
    }
}

fn populate(num_threads: usize) -> () {
    if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, STORAGE_DIR.to_string()) {
        let store = Arc::new(store);
        let mut threads = vec![];
        let num_active_threads = Arc::new(AtomicUsize::new(0));
        for thread_id in 0..num_threads {
            let store = Arc::clone(&store);
            let num_active_threads = Arc::clone(&num_active_threads);
            threads.push(std::thread::spawn(move || {
                // Populate Store
                let _session = store.start_session();
                num_active_threads.fetch_add(1, Ordering::SeqCst);

                for i in 0..NUM_OPS {
                    let idx = i as u64;
                    store.rmw(&(idx % NUM_UNIQUE_KEYS), &(1 as u64), idx);

                    if (idx % CHECKPOINT_INTERVAL == 0)
                        && num_active_threads.load(Ordering::SeqCst) == num_threads
                    {
                        let check = store.checkpoint().unwrap();
                        println!("Calling checkpoint with token {}", check.token);
                    }

                    if (idx % COMPLETE_PENDING_INTERVAL) == 0 {
                        store.complete_pending(false);
                    } else if (idx % REFRESH_INTERVAL) == 0 {
                        store.refresh();
                    }
                }

                store.complete_pending(true);
                store.stop_session();
                println!("Thread {} finished populating", thread_id);
            }));
        }
        for t in threads {
            t.join().expect("Something went wrong in a thread");
        }
        println!("Threads finished populating");
        println!("Store size: {}", store.size());
        println!("Verifying values");

        store.start_session();
        let mut read_results = Vec::with_capacity(NUM_UNIQUE_KEYS as usize);
        for idx in 0..NUM_UNIQUE_KEYS {
            let (_, receiver): (u8, Receiver<u64>) = store.read(&idx, idx);
            read_results.insert(idx as usize, receiver);
        }
        store.complete_pending(true);
        store.stop_session();

        let expected_value: u64 = (num_threads as u64) * NUM_OPS / NUM_UNIQUE_KEYS;
        for idx in 0..NUM_UNIQUE_KEYS {
            match read_results[idx as usize].recv() {
                Ok(val) => {
                    if val != expected_value {
                        println!(
                            "Error for {}, expected {}, actual {}",
                            idx, expected_value, val
                        );
                    }
                }
                Err(_) => {
                    println!("Error reading {}", idx);
                }
            }
        }
    } else {
        println!("Failed to create FasterKV store");
    }
}

fn recover(token: String, num_threads: usize) -> () {
    println!("Attempting to recover");
    if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, STORAGE_DIR.to_string()) {
        match store.recover(token.clone(), token.clone()) {
            Ok(rec) => {
                println!("Recover version: {}", rec.version);
                println!("Recover status: {}", rec.status);
                println!("Recovered sessions: {:?}", rec.session_ids);
                
                let mut serial_nums = vec![];
                for id in rec.session_ids {
                    serial_nums.push(store.continue_session(id));
                    store.stop_session();
                }

                store.start_session();
                let mut read_results = Vec::with_capacity(NUM_UNIQUE_KEYS as usize);
                for idx in 0..NUM_UNIQUE_KEYS {
                    let (_, receiver): (u8, Receiver<u64>) = store.read(&idx, idx);
                    read_results.insert(idx as usize, receiver);
                }
                store.complete_pending(true);
                store.stop_session();


                println!("Generating expected values");
                let mut expected_results = Vec::with_capacity(NUM_UNIQUE_KEYS as usize);
                expected_results.resize(NUM_UNIQUE_KEYS as usize, 0);

                // Sessions active during checkpoint
                for persisted_count in serial_nums.iter() {
                    for i in 0..(persisted_count + 1) {
                        let elem = expected_results
                            .get_mut((i % NUM_UNIQUE_KEYS) as usize)
                            .unwrap();
                        *elem += 1;
                    }
                }

                // Sessions completed before checkpoint
                for _ in 0..(num_threads - serial_nums.len()) {
                    let persisted_count = NUM_OPS;
                    for i in 0..persisted_count {
                        let elem = expected_results
                            .get_mut((i % NUM_UNIQUE_KEYS) as usize)
                            .unwrap();
                        *elem += 1;
                    }
                }

                println!("Verifying recovered values!");
                let mut incorrect = 0;
                for i in 0..NUM_OPS {
                    let idx = i as u64;
                    let (status, recv): (u8, Receiver<u64>) =
                        store.read(&(idx % NUM_UNIQUE_KEYS), idx);
                    if let Ok(val) = recv.recv() {
                        let expected = *expected_results
                            .get((idx % NUM_UNIQUE_KEYS) as usize)
                            .unwrap();
                        if expected != val {
                            println!(
                                "Error recovering {}, expected {}, got {}",
                                idx, expected, val
                            );
                            incorrect += 1;
                        }
                    } else {
                        println!("Failure to read with status: {}, and key: {}", status, idx);
                    }
                }
                println!("{} incorrect recoveries", incorrect);
            }
            Err(_) => println!("Recover operation failed"),
        }
    } else {
        println!("{}", "Failed to create recover store");
    }
}
