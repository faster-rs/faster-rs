extern crate bincode;
extern crate libc;
extern crate libfaster_sys as ffi;

use crate::status;

use bincode::deserialize;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::Sender;

pub trait FasterValue: Deserialize<'static> + Serialize {
    unsafe extern "C" fn read_callback<T>(
        sender: *mut libc::c_void,
        value: *const u8,
        length: u64,
        status: u32,
    ) where
        T: Deserialize<'static>,
    {
        let boxed_sender = Box::from_raw(sender as *mut Sender<T>);
        let sender = *boxed_sender;
        if status == status::OK.into() {
            let val = deserialize(std::slice::from_raw_parts(value, length as usize)).unwrap();
            // TODO: log error
            let _ = sender.send(val);
        }
    }

    unsafe extern "C" fn rmw_callback<T>(
        current: *const u8,
        length_current: u64,
        modification: *mut u8,
        length_modification: u64,
        dst: *mut u8,
    ) -> u64
    where
        T: Serialize + Deserialize<'static> + FasterValue,
    {
        let val: T =
            deserialize(std::slice::from_raw_parts(current, length_current as usize)).unwrap();
        let modif = deserialize(std::slice::from_raw_parts_mut(
            modification,
            length_modification as usize,
        ))
        .unwrap();
        let modified = val.rmw(modif);
        let encoded = bincode::serialize(&modified).unwrap();
        let size = encoded.len();
        if dst != std::ptr::null_mut() {
            encoded.as_ptr().copy_to(dst, size);
        }
        size as u64
    }

    fn rmw(&self, modification: Self) -> Self;
}
