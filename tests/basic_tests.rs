extern crate faster_rs;

use faster_rs::{status, FasterKv};
use std::collections::HashSet;
use std::sync::mpsc::Receiver;

fn add_numbers(val: &[u8], modification: &[u8]) -> Vec<u8> {
    let mut result = 0;
    result += bincode::deserialize::<u64>(val).unwrap();
    result += bincode::deserialize::<u64>(modification).unwrap();
    bincode::serialize(&result).unwrap()
}

fn join_vecs(val: &[u8], modification: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(val.len() + modification.len());
    result.extend_from_slice(val);
    result.extend_from_slice(modification);
    result
}

fn union_hashset(val: &[u8], modification: &[u8]) -> Vec<u8> {
    let mut result = HashSet::new();
    let val = bincode::deserialize::<HashSet<i32>>(val).unwrap();
    let modification = bincode::deserialize::<HashSet<i32>>(modification).unwrap();
    for e in val {
        result.insert(e);
    }
    for e in modification {
        result.insert(e);
    }
    bincode::serialize(&result).unwrap()
}

#[test]
fn faster_check() {
    let store = FasterKv::default();
    let key = bincode::serialize(&(1 as u64)).unwrap();
    let value = bincode::serialize(&(1337 as u64)).unwrap();

    let upsert = store.upsert(&key, &value, 1);
    assert!((upsert == status::OK || upsert == status::PENDING) == true);

    //let rmw = store.rmw(&key, bincode::serialize(&(5 as u64)).unwrap(), add_numbers, 1);
    //assert!(rmw == status::OK);

    assert!(store.size() > 0);
}

#[test]
fn faster_read_inserted_value() {
    let store = FasterKv::default();
    let key = bincode::serialize(&(1 as u64)).unwrap();
    let value = bincode::serialize(&(1337 as u64)).unwrap();

    let upsert = store.upsert(&key, &value, 1);
    assert!((upsert == status::OK || upsert == status::PENDING) == true);

    let (res, recv) = store.read(&key, 1);
    assert!(res == status::OK);
    let result = recv.recv().unwrap();
    assert_eq!(bincode::deserialize::<u64>(&result).unwrap(), 1337);
}

#[test]
fn faster_read_missing_value_recv_error() {
    let store = FasterKv::default();
    let key = bincode::serialize(&(1 as u64)).unwrap();

    let (res, recv) = store.read(&key, 1);
    assert!(res == status::NOT_FOUND);
    assert!(recv.recv().is_err());
}

#[test]
fn faster_rmw_changes_values() {
    let store = FasterKv::default();
    let key = bincode::serialize(&(1 as u64)).unwrap();
    let value = bincode::serialize(&(1337 as u64)).unwrap();
    let modification = bincode::serialize(&(100 as u64)).unwrap();

    let upsert = store.upsert(&key, &value, 1);
    assert!((upsert == status::OK || upsert == status::PENDING) == true);

    let (res, recv) = store.read(&key, 1);
    assert!(res == status::OK);
    let result = recv.recv().unwrap();
    assert_eq!(bincode::deserialize::<u64>(&result).unwrap(), 1337);

    let rmw = store.rmw(&key, &modification, add_numbers, 1);
    assert!((rmw == status::OK || rmw == status::PENDING) == true);

    let (res, recv) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    let result = recv.recv().unwrap();
    assert_eq!(bincode::deserialize::<u64>(&result).unwrap(), 1437);
}

#[test]
fn faster_rmw_without_upsert() {
    let store = FasterKv::default();
    let key = bincode::serialize(&(1 as u64)).unwrap();
    let modification = bincode::serialize(&(100 as u64)).unwrap();

    let rmw = store.rmw(&key, &modification, add_numbers, 1);
    assert!((rmw == status::OK || rmw == status::PENDING) == true);

    let (res, recv) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    let result = recv.recv().unwrap();
    assert_eq!(bincode::deserialize::<u64>(&result).unwrap(), 100);
}

#[test]
fn faster_rmw_string() {
    let store = FasterKv::default();
    let key = bincode::serialize(&(1 as u64)).unwrap();
    let value = String::from("Hello, ");
    let modification = String::from("World!");

    let upsert = store.upsert(&key, &value, 1);
    assert!(upsert == status::OK || upsert == status::PENDING);

    let rmw = store.rmw(&key, &modification, join_vecs, 1);
    assert!(rmw == status::OK || rmw == status::PENDING);

    let (res, recv) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    let result = recv.recv().unwrap();
    assert_eq!(
        String::from_utf8(result).unwrap(),
        String::from("Hello, World!")
    );
}

#[test]
fn faster_rmw_vec() {
    let store = FasterKv::default();
    let key = bincode::serialize(&(1 as u64)).unwrap();
    let value = vec![0, 1, 2];
    let modification = vec![3, 4, 5];
    let modification2 = vec![6, 7, 8, 9, 10];

    let upsert = store.upsert(&key, &value, 1);
    assert!(upsert == status::OK || upsert == status::PENDING);

    let rmw = store.rmw(&key, &modification, join_vecs, 1);
    assert!(rmw == status::OK || rmw == status::PENDING);

    let rmw = store.rmw(&key, &modification2, join_vecs, 1);
    assert!(rmw == status::OK || rmw == status::PENDING);

    let (res, recv) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    assert_eq!(recv.recv().unwrap(), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
}

#[test]
fn faster_rmw_grow_string() {
    let store = FasterKv::default();
    let key = String::from("growing_string");
    let final_string = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    for i in 0..final_string.len() {
        let letter = final_string.get(i..i + 1).unwrap();
        store.rmw(&key, &letter, join_vecs, 1);
    }

    let (res, recv) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    assert_eq!(
        String::from_utf8(recv.recv().unwrap()).unwrap(),
        final_string
    );
}

#[test]
fn faster_rmw_hashset() {
    let store = FasterKv::default();
    let key = String::from("set");
    {
        let a: HashSet<i32> = [1, 2, 3].iter().cloned().collect();
        store.rmw(&key, &bincode::serialize(&a).unwrap(), union_hashset, 1);
    }
    {
        let b: HashSet<i32> = [4, 2, 3, 4, 5].iter().cloned().collect();
        store.rmw(&key, &bincode::serialize(&b).unwrap(), union_hashset, 1);
    }
    let (res, recv) = store.read(&key, 1);
    assert_eq!(res, status::OK);
    let hash_set = bincode::deserialize::<HashSet<i32>>(&recv.recv().unwrap()).unwrap();
    assert_eq!(hash_set.len(), 5);
    assert!(hash_set.contains(&1));
    assert!(hash_set.contains(&2));
    assert!(hash_set.contains(&3));
    assert!(hash_set.contains(&4));
    assert!(hash_set.contains(&5));
}
