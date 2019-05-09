extern crate faster_rs;
extern crate tempfile;

use faster_rs::{status, FasterKv};
use std::sync::mpsc::Receiver;
use tempfile::TempDir;

const TABLE_SIZE: u64 = 1 << 14;
const LOG_SIZE: u64 = 17179869184;

#[test]
fn faster_check() {
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = FasterKv::new(TABLE_SIZE, LOG_SIZE, dir_path).unwrap();
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
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = FasterKv::new(TABLE_SIZE, LOG_SIZE, dir_path).unwrap();
    let key: u64 = 1;
    let value: u64 = 1337;

    let upsert = store.upsert(&key, &value, 1);
    assert!((upsert == status::OK || upsert == status::PENDING) == true);

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::OK);
    assert!(recv.recv().unwrap() == value);
}

#[test]
fn faster_read_missing_value_recv_error() {
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = FasterKv::new(TABLE_SIZE, LOG_SIZE, dir_path).unwrap();
    let key: u64 = 1;

    let (res, recv): (u8, Receiver<u64>) = store.read(&key, 1);
    assert!(res == status::NOT_FOUND);
    assert!(recv.recv().is_err());
}

#[test]
fn faster_rmw_changes_values() {
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = FasterKv::new(TABLE_SIZE, LOG_SIZE, dir_path).unwrap();
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
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = FasterKv::new(TABLE_SIZE, LOG_SIZE, dir_path).unwrap();
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
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = FasterKv::new(TABLE_SIZE, LOG_SIZE, dir_path).unwrap();
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
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = FasterKv::new(TABLE_SIZE, LOG_SIZE, dir_path).unwrap();
    let key: u64 = 1;
    let value = vec![0, 1, 2];
    let modification = vec![3, 4, 5];

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
}
