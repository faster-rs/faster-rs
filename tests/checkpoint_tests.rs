extern crate faster_kvs;
extern crate tempfile;

use faster_kvs::FasterKv;
use tempfile::TempDir;

#[test]
fn single_checkpoint() {
    let table_size: u64  = 1 << 14;
    let log_size: u64 = 17179869184;
    let tmp_dir = TempDir::new().unwrap();
    let dir_path = tmp_dir.path().to_string_lossy().into_owned();
    let store = FasterKv::new(table_size, log_size, dir_path).unwrap();
    let value: u64 = 100;

    for key in 0..1000 {
        store.upsert(key as u64, &value);
    }

    let checkpoint = store.checkpoint().unwrap();
    assert_eq!(checkpoint.checked, true);
    assert_eq!(checkpoint.token.len(), 37-1); // -1 \0
}


#[test]
fn concurrent_checkpoints() {
    //TODO
}
