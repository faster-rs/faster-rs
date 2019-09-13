extern crate faster_rs;

use faster_rs::{status, FasterKv};
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

#[test]
fn faster_check() {
    let store = FasterKv::default();
    let key: u64 = 1;
    let value: u64 = 1337;

    let upsert = store.upsert(&key, &value, 1);
    assert!((upsert == status::OK || upsert == status::PENDING) == true);

    let rmw = store.rmw(&key, &(5 as u64), 1);
    assert!(rmw == status::OK);

    assert!(store.size() > 0);
}

#[test]
fn faster_read_inserted_value() {
    let store = FasterKv::default();
    let key: u64 = 1;
    let value: u64 = 1337;

    let upsert = store.upsert(&key, &value, 1);
    assert!((upsert == status::OK || upsert == status::PENDING) == true);

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::OK);
    assert!(recv.recv().unwrap() == value);
}

#[test]
fn faster_delete_inserted_value() {
    let store = FasterKv::default();
    let key: u64 = 1;
    let value: u64 = 1337;

    let upsert = store.upsert(&key, &value, 1);
    assert!((upsert == status::OK || upsert == status::PENDING) == true);

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::OK);
    assert!(recv.recv().unwrap() == value);

    let delete = store.delete(&key, 1);
    assert!((delete == status::OK || delete == status::PENDING) == true);

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::NOT_FOUND);
    assert!(recv.recv().is_err());
}

#[test]
fn faster_read_missing_value_recv_error() {
    let store = FasterKv::default();
    let key: u64 = 1;

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::NOT_FOUND);
    assert!(recv.recv().is_err());
}

#[test]
fn faster_rmw_changes_values() {
    let store = FasterKv::default();
    let key: u64 = 1;
    let value: u64 = 1337;
    let modification: u64 = 100;

    let upsert = store.upsert(&key, &value, 1);
    assert!((upsert == status::OK || upsert == status::PENDING) == true);

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::OK);
    assert!(recv.recv().unwrap() == value);

    let rmw = store.rmw(&key, &modification, 1);
    assert!((rmw == status::OK || rmw == status::PENDING) == true);

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::OK);
    assert!(recv.recv().unwrap() == value + modification);
}

#[test]
fn faster_rmw_without_upsert() {
    let store = FasterKv::default();
    let key: u64 = 1;
    let modification: u64 = 100;

    let rmw = store.rmw(&key, &modification, 1);
    assert!((rmw == status::OK || rmw == status::PENDING) == true);

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::OK);
    assert!(recv.recv().unwrap() == modification);
}

#[test]
fn faster_rmw_string() {
    let store = FasterKv::default();
    let key: u64 = 1;
    let value = String::from("Hello, ");
    let modification = String::from("World!");

    let upsert = store.upsert(&key, &value, 1);
    assert!(upsert == status::OK || upsert == status::PENDING);

    let (res, recv): (u8, Receiver<String>) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    assert_eq!(recv.recv().unwrap(), value);

    let rmw = store.rmw(&key, &modification, 1);
    assert!(rmw == status::OK || rmw == status::PENDING);

    let (res, recv): (u8, Receiver<String>) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    assert_eq!(recv.recv().unwrap(), String::from("Hello, World!"));
}

#[test]
fn faster_rmw_vec() {
    let store = FasterKv::default();
    let key: u64 = 1;
    let value = vec![0, 1, 2];
    let modification = vec![3, 4, 5];
    let modification2 = vec![6, 7, 8, 9, 10];

    let upsert = store.upsert(&key, &value, 1);
    assert!(upsert == status::OK || upsert == status::PENDING);

    let (res, recv): (u8, Receiver<Vec<i32>>) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    assert_eq!(recv.recv().unwrap(), value);

    let rmw = store.rmw(&key, &modification, 1);
    assert!(rmw == status::OK || rmw == status::PENDING);

    let (res, recv): (u8, Receiver<Vec<i32>>) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    assert_eq!(recv.recv().unwrap(), vec![0, 1, 2, 3, 4, 5]);

    let rmw = store.rmw(&key, &modification2, 1);
    assert!(rmw == status::OK || rmw == status::PENDING);

    let (res, recv): (u8, Receiver<Vec<i32>>) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    assert_eq!(recv.recv().unwrap(), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn faster_rmw_grow_string() {
    let store = FasterKv::default();
    let key = String::from("growing_string");
    let final_string = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    for i in 0..final_string.len() {
        let letter: String = final_string.get(i..i + 1).unwrap().to_string();
        store.rmw(&key, &letter, 1);
    }

    let (res, recv): (u8, Receiver<String>) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    assert_eq!(recv.recv().unwrap(), final_string);
}

#[test]
fn faster_rmw_hashset() {
    let store = FasterKv::default();
    let key = String::from("set");
    {
        let a: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
        store.rmw(&key, &a, 1);
    }
    {
        let b: HashSet<i32> = [4, 2, 3, 4, 5].iter().cloned().collect();
        store.rmw(&key, &b, 1);
    }
    let (res, recv): (u8, Receiver<HashSet<i32>>) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    let hash_set = recv.recv().unwrap();
    assert_eq!(hash_set.len(), 5);
    assert!(hash_set.contains(&1));
    assert!(hash_set.contains(&2));
    assert!(hash_set.contains(&3));
    assert!(hash_set.contains(&4));
    assert!(hash_set.contains(&5));
}
