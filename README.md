# Experimental FASTER wrapper for Rust

Includes experimental C interface for FASTER. It currently assumes the KEY,VALUE types are u64. This wrapper is only focusing on Linux support. 


It is probably a good idea to make sure you can compile the C++ version before you start playing around with this wrapper.


Down below are some example operations. 

```rust,no_run
extern crate faster_kvs;

use faster_kvs::*;

const TABLE_SIZE: u64  = 1 << 14;
const LOG_SIZE: u64 = 17179869184;

fn main() {
  if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("storage_dir")) {
    let key: u64 = 1;
    let value: u64 = 1000;

    // Upsert
    store.upsert(key, value);


    // Read-Modify-Write
    let incr: u64 = 50;
    let rmw = store.rmw(key, incr);
    assert_eq!(rmw, status::OK);


    // Read
    let (status, recv) = store.read(key);
    assert_eq!(read, status::OK);
    assert_eq!(recv.recv().unwrap(), value);

    let bad_key: u64 = 2;
    let bad_read = store.read(bad_key);
    assert_eq!(bad_read, status::NOT_FOUND);
  }
}
```

# Things to fix

- [x] Fix so you can actually return the values from read
- [ ] Experiment with #repr(C) structs for values rather than u64
- [ ] Look into threading and async callbacks into Rust
- [ ] Finish off the rest off the operations in the C interface
- [ ] Compare performance to C++ version
