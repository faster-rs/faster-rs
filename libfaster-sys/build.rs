extern crate cc;
extern crate bindgen;

use std::env;
use std::fs;
use std::path::PathBuf;

fn build_faster() {
    let mut config = cc::Build::new();

    config.include("FASTER/cc/src/");
    config.compile("fasterc");
}

// https://github.com/rust-rocksdb/rust-rocksdb/blob/master/librocksdb-sys/build.rs
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

fn try_to_find_and_link_lib(lib_name: &str) -> bool {
    if let Ok(lib_dir) = env::var(&format!("{}_LIB_DIR", lib_name)) {
        println!("cargo:rustc-link-search=native={}", lib_dir);
        let mode = match env::var_os(&format!("{}_STATIC", lib_name)) {
            Some(_) => "static",
            None => "dylib",
        };
        println!("cargo:rustc-link-lib={}={}", mode, lib_name.to_lowercase());
        return true;
    }
    false
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=FASTER/");

    fail_on_empty_directory("FASTER");
    faster_bindgen();

    if !try_to_find_and_link_lib("FASTER") {
        build_faster();
    } else {
        println!("NEJ");
    }
}
