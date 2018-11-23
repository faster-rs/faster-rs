#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

extern crate libc;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use libc::*;
    use std::ffi::{CStr, CString};
    use std::ptr;
    use std::str;
    use super::*;

    #[test]
    fn simple() {
        unsafe {
            let storage = CString::new("storage").unwrap();
            let key_space: size_t = 1 << 14;
            let store = faster_open_with_disk(storage.as_ptr(), key_space);
            assert!(store.is_null());
        }
    }
}
