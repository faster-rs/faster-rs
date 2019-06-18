[![License](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/faster-rs/faster-rs)
[![Cargo](https://img.shields.io/crates/v/faster-rs.svg)](https://crates.io/crates/faster-rs)
[![Build Status](https://dev.azure.com/faster-rs/faster-rs/_apis/build/status/faster-rs.faster-rs?branchName=master)](https://dev.azure.com/faster-rs/faster-rs/_build/latest?definitionId=1&branchName=master)

# Experimental FASTER wrapper for Rust

```toml
[dependencies]
faster-rs = "0.6.0"
```

Includes experimental C interface for FASTER. It is a generic implementation of FASTER that allows arbitrary Key-Value pairs to be stored. This wrapper is only focusing on Linux support.

Install Dependencies (Ubuntu):
```
$ add-apt-repository -y ppa:ubuntu-toolchain-r/test
$ apt update
$ apt install -y g++-7 libaio-dev uuid-dev libtbb-dev
```

*Make sure you clone the submodules as well*, this is best done by cloning with `git clone --recurse-submodules`.

## The interface
This wrapper attempts to remain true to the original FASTER design by exposing a similar interface to that which is provided by the original C++ version. Users may define their own Key-Value types (that implement the `FasterKey` and `FasterValue` traits) and provide custom logic for Read-Modify-Write operations.


The `Read`, `Upsert` and `RMW` operations all require a monotonic serial number to form the sequence of operations that will be persisted by FASTER. `Read` operations require a serial number so that at a CPR checkpoint boundary, FASTER guarantees that the reads before that point have accessed no data updates after the checkpoint. If persistence is not important, the serial number can safely be set to `1` for all operations (as is done in the examples above).

More information about Checkpointing and Recovery is provided below the following examples.

## A basic example

The following example shows the creation of a FASTER Key-Value Store and basic operations on `u64` values.

Try it out by running `cargo run --example basic`.

```rust,no_run
extern crate faster_rs;

use faster_rs::{status, FasterKv};
use std::sync::mpsc::Receiver;

fn main() {
    const TABLE_SIZE: u64 = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;

    // Create a Key-Value Store
    if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("example_basic_storage")) {
        let key0: u64 = 1;
        let value0: u64 = 1000;
        let modification: u64 = 5;

        // Upsert
        for i in 0..1000 {
            let upsert = store.upsert(&(key0 + i), &(value0 + i), i);
            assert!(upsert == status::OK || upsert == status::PENDING);
        }

        // Read-Modify-Write
        for i in 0..1000 {
            let rmw = store.rmw(&(key0 + i), &(5 as u64), i + 1000);
            assert!(rmw == status::OK || rmw == status::PENDING);
        }

        assert!(store.size() > 0);

        // Read
        for i in 0..1000 {
            // Note: need to provide type annotation for the Receiver
            let (read, recv): (u8, Receiver<u64>) = store.read(&(key0 + i), i);
            assert!(read == status::OK || read == status::PENDING);
            let val = recv.recv().unwrap();
            assert_eq!(val, value0 + i + modification);
            println!("Key: {}, Value: {}", key0 + i, val);
        }

        // Clear used storage
        match store.clean_storage() {
            Ok(()) => {}
            Err(_err) => panic!("Unable to clear FASTER directory"),
        }
    } else {
        panic!("Unable to create FASTER directory");
    }
}
```

## Using custom keys
`struct`s that can be (de)serialised using [serde](https://crates.rs/crates/serde) are supported as keys. In order to use such a `struct`, it is necessary to derive the implementations of `Serializable` and `Deserializable` from `serde-derive`. All types implementing these two traits will automatically implement `FasterKey` and thus be usable as a Key.

The following example shows a basic struct being used as a key. Try it out by running `cargo run --example custom_keys`.

```rust,no-run
extern crate faster_rs;
extern crate serde_derive;

use faster_rs::{status, FasterKv};
use serde_derive::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;

// Note: Debug annotation is just for printing later
#[derive(Serialize, Deserialize, Debug)]
struct MyKey {
    foo: String,
    bar: String,
}

fn main() {
    const TABLE_SIZE: u64 = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;

    // Create a Key-Value Store
    if let Ok(store) = FasterKv::new(
        TABLE_SIZE,
        LOG_SIZE,
        String::from("example_custom_values_storage"),
    ) {
        let key = MyKey {
            foo: String::from("Hello"),
            bar: String::from("World"),
        };
        let value: u64 = 1;

        // Upsert
        let upsert = store.upsert(&key, &value, 1);
        assert!(upsert == status::OK || upsert == status::PENDING);

        assert!(store.size() > 0);

        // Note: need to provide type annotation for the Receiver
        let (read, recv): (u8, Receiver<u64>) = store.read(&key, 1);
        assert!(read == status::OK || read == status::PENDING);
        let val = recv.recv().unwrap();
        println!("Key: {:?}, Value: {}", key, val);

        // Clear used storage
        match store.clean_storage() {
            Ok(()) => {}
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
extern crate faster_rs;
extern crate serde_derive;

use faster_rs::{status, FasterKv, FasterValue};
use serde_derive::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;

// Note: Debug annotation is just for printing later
#[derive(Serialize, Deserialize, Debug)]
struct MyValue {
    foo: String,
    bar: String,
}

impl FasterValue for MyValue {
    fn rmw(&self, _modification: MyValue) -> MyValue {
        unimplemented!()
    }
}

fn main() {
    const TABLE_SIZE: u64 = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;

    // Create a Key-Value Store
    if let Ok(store) = FasterKv::new(
        TABLE_SIZE,
        LOG_SIZE,
        String::from("example_custom_values_storage"),
    ) {
        let key: u64 = 1;
        let value = MyValue {
            foo: String::from("Hello"),
            bar: String::from("World"),
        };

        // Upsert
        let upsert = store.upsert(&key, &value, 1);
        assert!(upsert == status::OK || upsert == status::PENDING);

        assert!(store.size() > 0);

        // Note: need to provide type annotation for the Receiver
        let (read, recv): (u8, Receiver<MyValue>) = store.read(&key, 1);
        assert!(read == status::OK || read == status::PENDING);
        let val = recv.recv().unwrap();
        println!("Key: {}, Value: {:?}", key, val);

        // Clear used storage
        match store.clean_storage() {
            Ok(()) => {}
            Err(_err) => panic!("Unable to clear FASTER directory"),
        }
    } else {
        panic!("Unable to create FASTER directory");
    }
}
```

## Out-of-the-box implementations of `FasterValue`
Several types already implement `FasterValue` along with providing Read-Modify-Write logic. The implementations can be found in `src/impls.rs` but their RMW logic is summarised here:
* Numeric types use addition
* Bools and Chars replace old value for new value
* Strings and Vectors append new values (use an `upsert` to replace entire value)

## Checkpoint and Recovery
FASTER's fault tolerance is provided by [Concurrent Prefix Recovery](https://www.microsoft.com/en-us/research/uploads/prod/2019/01/cpr-sigmod19.pdf) (CPR). It provides the following semantics:
 > If operation X is persisted, then all operations before X in the input operation sequence are persisted as well (and none after).

Persisting operations is done using the `checkpoint()` function. It is also important to periodically call the `refresh()` function as it is the mechanism threads use to report forward progress to the system.

Individual sessions (threads accessing FASTER) will persist a different number of operations. The most recently persisted serial number is returned by the `continue_session()` function and allows reasoning about which operations were (not) persisted. It is also the operation sequence number from which the thread should continue to provide operations after recovery. 

A good demonstration of checkpointing/recovery can be found in `examples/sum_store_single.rs`. Try it out for yourself!
```bash
$ cargo run --example sum_store_single -- populate
$ cargo run --example sum_store_single -- recover <checkpoint-token>
```

## Benchmarking
It is possible to benchmark both the C-wrapper and the Rust-wrapper of FASTER. To build and run the C-benchmark follow Microsoft's instructions [here](https://github.com/Microsoft/FASTER/tree/master/cc) and then run the binary `benchmark-c`. It takes the same parameters and input format as the original benchmark.

### Running the Rust benchmark
The benchmark is written as a separate crate in the `benchmark` directory. Inside the directory run `cargo run --release -- help` to see the available options.

The benchmark consists of two subcommands `cargo run --release -- [process-ycsb|run]`:
* `process-ycsb` will take the output of the supplied YCSB file and produce an output file containing only the 8-byte key in the format expected by the Rust & C benchmarks
* `run` will actually execute the benchmark using the supplied load and run keys. The workload and number of threads can be customised.

The benchmark is very similar to the original C++ implementation so it's best to follow their instructions for setting up YCSB.
