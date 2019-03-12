extern crate bincode;
extern crate libc;
extern crate libfaster_sys as ffi;

use crate::status;

use bincode::deserialize;
use serde::{Deserialize, Serialize};
use std::mem;
use std::sync::mpsc::Sender;

pub trait FasterValue<'a, T: Deserialize<'a> + Serialize + FasterValue<'a, T>> {
    extern fn read_callback(sender: *mut libc::c_void, value: *mut u8, length: u64, status: u32) {
        let boxed_sender = unsafe {Box::from_raw(sender as *mut Sender<T>)};
        let sender = *boxed_sender;
        if status == status::OK.into() {
            let slice = unsafe {
                deserialize(std::slice::from_raw_parts_mut(value, length as usize)).unwrap()
            };
            sender.send(slice).unwrap();
        }
    }

    extern fn rmw_callback(
        value: *mut u8,
        modification: *mut u8,
        length_value: u64,
        length_modification: u64
    ) -> ffi::faster_rmw_result {
        let val: T = unsafe {
            deserialize(std::slice::from_raw_parts_mut(value, length_value as usize)).unwrap()
        };
        let modif = unsafe {
            deserialize(std::slice::from_raw_parts_mut(modification, length_modification as usize)).unwrap()
        };
        let modified = val.rmw(modif);
        let mut encoded = bincode::serialize(&modified).unwrap();
        let ptr = encoded.as_mut_ptr();
        let size = encoded.len();
        mem::forget(encoded);
        ffi::faster_rmw_result {
            value: ptr,
            size: size as u64,
        }
    }

    fn rmw(&self, modification: T) -> T;
}

impl <'a> FasterValue<'a, String> for String {
    fn rmw(&self, modification: String) -> String {
        unimplemented!()
    }
}

impl <'a> FasterValue<'a, u64> for u64 {
    fn rmw(&self, modification: u64) -> u64 {
        self + modification
    }
}
