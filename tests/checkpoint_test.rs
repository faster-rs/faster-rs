extern crate faster_kvs;
extern crate uuid;

use faster_kvs::FasterKv;
use uuid::Uuid;

#[test]
fn single_checkpoint() {
    let table_size: u64  = 1 << 14;
    let log_size: u64 = 17179869184;
    let store = FasterKv::new(table_size, log_size, String::from("hej")).unwrap();
    let _token = Uuid::new_v4();
    store.clean_storage()
        .expect("Failed cleaning storage");
    // Todo when checkpoint is fixed
}
