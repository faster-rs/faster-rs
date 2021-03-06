extern crate faster_rs;
extern crate tempfile;

use faster_rs::{FasterError, FasterKv, FasterKvBuilder};
use tempfile::TempDir;

#[test]
fn single_checkpoint() {
    let table_size: u64 = 1 << 14;
    let log_size: u64 = 1073741824;
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let mut builder = FasterKvBuilder::new(table_size, log_size);
    builder.with_disk(&dir_path);
    let store = builder.build().unwrap();
    let value: u64 = 100;

    for key in 0..1000 {
        store.upsert(&(key as u64), &value, key);
    }

    let checkpoint = store.checkpoint().unwrap();
    assert_eq!(checkpoint.checked, true);
    assert_eq!(checkpoint.token.len(), 37 - 1); // -1 \0
}

#[test]
fn single_checkpoint_index() {
    let table_size: u64 = 1 << 14;
    let log_size: u64 = 1073741824;
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let mut builder = FasterKvBuilder::new(table_size, log_size);
    builder.with_disk(&dir_path);
    let store = builder.build().unwrap();
    let value: u64 = 100;

    for key in 0..1000 {
        store.upsert(&(key as u64), &value, key);
    }

    let checkpoint = store.checkpoint_index().unwrap();
    assert_eq!(checkpoint.checked, true);
    assert_eq!(checkpoint.token.len(), 37 - 1); // -1 \0
}

#[test]
fn single_checkpoint_hybrid_log() {
    let table_size: u64 = 1 << 14;
    let log_size: u64 = 1073741824;
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let mut builder = FasterKvBuilder::new(table_size, log_size);
    builder.with_disk(&dir_path);
    let store = builder.build().unwrap();
    let value: u64 = 100;

    for key in 0..1000 {
        store.upsert(&(key as u64), &value, key);
    }

    let checkpoint = store.checkpoint_hybrid_log().unwrap();
    assert_eq!(checkpoint.checked, true);
    assert_eq!(checkpoint.token.len(), 37 - 1); // -1 \0
}

#[test]
fn concurrent_checkpoints() {
    //TODO
}

#[test]
fn in_memory_checkpoint_errors() {
    let store = FasterKv::default();
    let value: u64 = 100;

    for key in 0..1000 {
        store.upsert(&(key as u64), &value, key);
    }

    let checkpoint = store.checkpoint();
    assert!(checkpoint.is_err(), "Checkpoint should fail");
    match checkpoint.err().unwrap() {
        FasterError::InvalidType => assert!(true),
        _ => assert!(false, "Should give InvalidType Error"),
    }
}

#[test]
fn in_memory_checkpoint_index_errors() {
    let store = FasterKv::default();
    let value: u64 = 100;

    for key in 0..1000 {
        store.upsert(&(key as u64), &value, key);
    }

    let checkpoint = store.checkpoint_index();
    assert!(checkpoint.is_err(), "Checkpoint should fail");
    match checkpoint.err().unwrap() {
        FasterError::InvalidType => assert!(true),
        _ => assert!(false, "Should give InvalidType Error"),
    }
}

#[test]
fn in_memory_checkpoint_hybrid_log_errors() {
    let store = FasterKv::default();
    let value: u64 = 100;

    for key in 0..1000 {
        store.upsert(&(key as u64), &value, key);
    }

    let checkpoint = store.checkpoint_hybrid_log();
    assert!(checkpoint.is_err(), "Checkpoint should fail");
    match checkpoint.err().unwrap() {
        FasterError::InvalidType => assert!(true),
        _ => assert!(false, "Should give InvalidType Error"),
    }
}
