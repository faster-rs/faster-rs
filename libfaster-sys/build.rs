extern crate bindgen;

use std::env;
use std::fs;
use std::path::PathBuf;
use cmake::Config;

// Credit to: https://github.com/rust-rocksdb/rust-rocksdb/blob/master/librocksdb-sys/build.rs
fn fail_on_empty_directory(name: &str) {
    if fs::read_dir(name).unwrap().count() == 0 {
        println!(
            "The `{}` directory is empty, did you forget to pull the submodules?",
            name
        );
        println!("Try `git submodule update --init --recursive`");
        panic!();
    }
}

fn faster_bindgen() {
    let bindings = bindgen::Builder::default()
        .header("FASTER/cc/src/core/faster-c.h")
        .blacklist_type("max_align_t") // https://github.com/rust-lang-nursery/rust-bindgen/issues/550
        .ctypes_prefix("libc")
        .generate()
        .expect("unable to generate faster bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("unable to write faster bindings");
}


fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=FASTER/");

    fail_on_empty_directory("FASTER");

    faster_bindgen();

    let dst = Config::new("FASTER/cc")
        .cflag("--std=c++11 ")
        .build();

    println!("cargo:rustc-link-search=native={}/{}", dst.display(), "build");
    // Fix this...
    println!("cargo:rustc-link-lib=static=faster");
    println!("cargo:rustc-link-lib=stdc++fs");
    println!("cargo:rustc-link-lib=uuid");
    println!("cargo:rustc-link-lib=tbb");
    println!("cargo:rustc-link-lib=gcc");
    println!("cargo:rustc-link-lib=stdc++");
    println!("cargo:rustc-link-lib=aio");
    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
}
