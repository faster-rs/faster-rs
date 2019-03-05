extern crate faster_kvs;

use faster_kvs::*;
use std::env;
use std::sync::mpsc::Receiver;

const TABLE_SIZE: u64  = 1 << 15;
const LOG_SIZE: u64 = 17179869184;
const NUM_OPS: u64 = 1  << 25;
const NUM_UNIQUE_KEYS: u64 = 1 << 23;
const REFRESH_INTERVAL: u64 = 1 << 8;
const COMPLETE_PENDING_INTERVAL: u64 = 1 << 12;
const CHECKPOINT_INTERVAL: u64 = 1 << 20;

const STORAGE_DIR: &str = "single_threaded_recovery_storage";

// More or less a copy of the single-threaded sum_store populate/recover example from FASTER

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let operation = &args[1].to_string();

        if operation == "populate" {
            println!(
                "{}",
                "This may take a while, and make sure you have disk space"
            );
            populate();
        } else if operation == "recover" {
            if args.len() > 2 {
                let token = &args[2];
                recover(token.to_string());
            } else {
                println!("Second argument required is token checkpoint to recover");
            }
        }
    } else {
        println!("Populate: args {}", "1. populate");
        println!("Recover: args {}, {}", "1. recover", "2. checkpoint token");
    }
}

fn populate() -> () {
    if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, STORAGE_DIR.to_string()) {
        // Populate Store
        let session = store.start_session();
        println!("Starting Session {}", session);

        for i in 0..NUM_OPS {
            let idx = i as u64;
            store.rmw(idx % NUM_UNIQUE_KEYS, 1);

            if (idx % CHECKPOINT_INTERVAL) == 0 {
                let check = store.checkpoint().unwrap();
                println!("Calling checkpoint with token {}", check.token);
            }

            if (idx % COMPLETE_PENDING_INTERVAL) == 0 {
                store.complete_pending(false);
            } else if (idx % REFRESH_INTERVAL) == 0 {
                store.refresh();
            }
        }

        println!("Ensuring values stored correctly");
        let mut expected_results = Vec::with_capacity(NUM_UNIQUE_KEYS as usize);
        expected_results.resize(NUM_UNIQUE_KEYS as usize, 0);
        for i in 0..NUM_OPS {
            let elem =  expected_results.get_mut((i % NUM_UNIQUE_KEYS) as usize).unwrap();
            *elem += 1;
        }

        for i in 0..NUM_OPS {
            let idx = i as u64;
            let (status, recv)= store.read(idx % NUM_UNIQUE_KEYS);
            if let Ok(val) = recv.recv() {
                assert_eq!(val, *expected_results.get((idx % NUM_UNIQUE_KEYS) as usize).unwrap(), "Failed to read: {}", idx);
            } else {
                println!("Failure to read with status: {}, and key: {}", status, idx);
            }
        }

        println!("Stopping Session {}", session);
        store.complete_pending(true);
        store.stop_session();
        println!("Store size: {}", store.size());
    } else {
        println!("Failed to create FasterKV store");
    }
}

fn recover(token: String) -> () {
    println!("Attempting to recover");
    if let Ok(recover_store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, STORAGE_DIR.to_string()) {
        match recover_store.recover(token.clone(), token.clone()) {
            Some(rec) => {
                println!("Recover version: {}", rec.version);
                println!("Recover status: {}", rec.status);
                println!("Recovered sessions: {:?}", rec.session_ids);
                recover_store.continue_session(rec.session_ids.first().cloned().unwrap());

                let mut expected_results = Vec::with_capacity(NUM_UNIQUE_KEYS as usize);
                expected_results.resize(NUM_UNIQUE_KEYS as usize, 0);
                for i in 0..NUM_OPS {
                    let elem =  expected_results.get_mut((i % NUM_UNIQUE_KEYS) as usize).unwrap();
                    *elem += 1;
                }

                println!("Verifying recovered values!");
                let mut incorrect = 0;
                for i in 0..NUM_OPS {
                    let idx = i as u64;
                    let (status, recv)= recover_store.read(idx % NUM_UNIQUE_KEYS);
                    if let Ok(val) = recv.recv() {
                        let expected = *expected_results.get((idx % NUM_UNIQUE_KEYS) as usize).unwrap();
                        if expected != val {
                            println!("Error recovering {}, expected {}, got {}", idx, expected, val);
                            incorrect += 1;
                        }
                    } else {
                        println!("Failure to read with status: {}, and key: {}", status, idx);
                    }
                }
                println!("{} incorrect recoveries", incorrect);
                println!("Ok.....!");
                recover_store.stop_session();
            },
            None => println!("Recover operation failed"),
        }
    } else {
        println!("{}", "Failed to create recover store");
    }
}
