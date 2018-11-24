#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

extern crate libc;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use super::*;

    const TABLE_SIZE: u64  = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;

    #[test]
    fn store_creation() {
        unsafe {
            let storage = CString::new("storage_dir").unwrap();
            let store = faster_open_with_disk(TABLE_SIZE, LOG_SIZE, storage.as_ptr());
            assert!(!store.is_null());
            faster_destroy(store);
        }
    }

    #[test]
    fn write_read_request() {
        unsafe {
            let storage = CString::new("storage_dir").unwrap();
            let store = faster_open_with_disk(TABLE_SIZE, LOG_SIZE, storage.as_ptr());
            assert!(!store.is_null());
            let key: u64 = 1;
            let value: u64 = 1000;
            faster_upsert(store, key, value);
            let res: u8 = faster_read(store, key);
            assert!(res == 0); // 0 == Ok

            let bad_key: u64 = 2;
            let bad_res: u8 = faster_read(store, bad_key);
            assert!(bad_res == 2); // 2 == NotFound

            faster_destroy(store);
        }
    }

}
