# Experimental FASTER wrapper for Rust

Includes experimental C interface for FASTER. It currently assumes the KEY type is u64 however the VALUE type supports arbitrary serialisable structs. This wrapper is only focusing on Linux support.


It is probably a good idea to make sure you can compile the C++ version before you start playing around with this wrapper.

*Make sure you clone the submodules as well*, this is best done by cloning with `git clone --recurse-submodules`.


## A basic example

The following example shows the creation of a FASTER Key-Value Store and basic operations on `u64` values.

Try it out by running `cargo run --example basic`.

```rust,no_run
extern crate faster_kvs;

use faster_kvs::{FasterKv, status};
use std::sync::mpsc::Receiver;

fn main() {
    const TABLE_SIZE: u64  = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;

    // Create a Key-Value Store
    if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("example_basic_storage")) {
        let key0: u64 = 1;
        let value0: u64 = 1000;
        let modification: u64 = 5;

        // Upsert
        for i in 0..1000 {
            let upsert = store.upsert(key0 + i, &(value0 + i));
            assert!(upsert == status::OK || upsert == status::PENDING);
        }

        // Read-Modify-Write
        for i in 0..1000 {
            let rmw = store.rmw(key0 + i, &(5 as u64));
            assert!(rmw == status::OK || rmw == status::PENDING);
        }

        assert!(store.size() > 0);

        // Read
        for i in 0..1000 {
            // Note: need to provide type annotation for the Receiver
            let (read, recv): (u8, Receiver<u64>) = store.read(key0 + i);
            assert!(read == status::OK || read == status::PENDING);
            let val = recv.recv().unwrap();
            assert_eq!(val, value0 + i + modification);
            println!("Key: {}, Value: {}", key0 + i, val);
        }

        // Clear used storage
        match store.clean_storage() {
            Ok(()) => {},
            Err(_err) => panic!("Unable to clear FASTER directory"),
        }
    } else {
        panic!("Unable to create FASTER directory");
    }
}
```

## Using custom values
`struct`s that can be (de)serialised using [serde](https://crates.rs/crates/serde) are supported as values. In order to use such a `struct`, it is necessary to derive the implementations of `Serializable` and `Deserializable` from `serde-derive`. It is also necessary to implement the `FasterValue` trait which exposes an `rmw()` function. This function can be used to implement custom logic for Read-Modify-Write operations or simply left with an `unimplemented!()` macro. In the latter case, any attempt to invoke a RMW operation will cause a panic.

The following example shows a basic struct being used as a value. Try it out by running `cargo run --example custom_values`.

```rust,no_run
extern crate faster_kvs;
extern crate serde_derive;

use faster_kvs::{FasterKv, FasterValue,status};
use serde_derive::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;

// Note: Debug annotation is just for printing later
#[derive(Serialize, Deserialize, Debug)]
struct MyValue {
    foo: String,
    bar: String,
}

impl FasterValue<'_, MyValue> for MyValue {
    fn rmw(&self, modification: MyValue) -> MyValue {
        unimplemented!()
    }
}

fn main() {
    const TABLE_SIZE: u64  = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;

    // Create a Key-Value Store
    if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("example_custom_values_storage")) {
        let key: u64 = 1;
        let value = MyValue { foo: String::from("Hello"), bar: String::from("World") };

        // Upsert
        let upsert = store.upsert(key, &value);
        assert!(upsert == status::OK || upsert == status::PENDING);

        assert!(store.size() > 0);

        // Note: need to provide type annotation for the Receiver
        let (read, recv): (u8, Receiver<MyValue>) = store.read(key);
        assert!(read == status::OK || read == status::PENDING);
        let val = recv.recv().unwrap();
        println!("Key: {}, Value: {:?}", key, val);

        // Clear used storage
        match store.clean_storage() {
            Ok(()) => {},
            Err(_err) => panic!("Unable to clear FASTER directory"),
        }
    } else {
        panic!("Unable to create FASTER directory");
    }
}
```

# Things to fix

- [x] Fix so you can actually return the values from read
- [ ] Experiment with #repr(C) structs for values rather than u64
- [ ] Look into threading and async callbacks into Rust
- [ ] Finish off the rest off the operations in the C interface
- [ ] Compare performance to C++ version
