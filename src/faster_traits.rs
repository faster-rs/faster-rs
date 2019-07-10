extern crate bincode;
extern crate libc;
extern crate libfaster_sys as ffi;

use crate::status;

use bincode::deserialize;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::mpsc::Sender;

pub trait FasterKey: DeserializeOwned + Serialize {}

pub trait FasterValue: DeserializeOwned + Serialize {}

#[inline(always)]
pub unsafe extern "C" fn read_callback<T>(
    sender: *mut libc::c_void,
    value: *const u8,
    length: u64,
    status: u32,
) where
    T: DeserializeOwned,
{
    let boxed_sender = Box::from_raw(sender as *mut Sender<T>);
    let sender = *boxed_sender;
    if status == status::OK.into() {
        let val = deserialize(std::slice::from_raw_parts(value, length as usize)).unwrap();
        // TODO: log error
        let _ = sender.send(val);
    }
}

#[inline(always)]
pub unsafe extern "C" fn rmw_callback<T>(
    current: *const u8,
    length_current: u64,
    modification: *mut u8,
    length_modification: u64,
    dst: *mut u8,
) -> u64
where
    T: Serialize + DeserializeOwned + FasterRmw,
{
    let val: T = deserialize(std::slice::from_raw_parts(current, length_current as usize)).expect("Couldn't deserialise value");
    let modif: T = deserialize(std::slice::from_raw_parts_mut(
        modification,
        length_modification as usize,
    ))
    .expect("Couldn't deserialise modification");
    let modified: T = val.rmw(modif);
    let encoded = bincode::serialize(&modified).unwrap();
    let size = encoded.len();
    if dst != std::ptr::null_mut() {
        encoded.as_ptr().copy_to(dst, size);
    }
    size as u64
}

pub trait FasterRmw: DeserializeOwned + Serialize {
    /// Specify custom Read-Modify-Write logic
    ///
    /// # Example
    /// ```
    /// use faster_rs::{status, FasterKv, FasterRmw};
    /// use serde_derive::{Deserialize, Serialize};
    /// use std::sync::mpsc::Receiver;
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct MyU64 {
    ///     value: u64,
    /// }
    /// impl FasterRmw for MyU64 {
    ///     fn rmw(&self, modification: Self) -> Self {
    ///         MyU64 {
    ///             value: self.value + modification.value,
    ///         }
    ///     }
    /// }
    ///
    /// let store = FasterKv::new_in_memory(32768, 536870912);
    /// let key = 5 as u64;
    /// let value = MyU64 { value: 12 };
    /// let modification = MyU64 { value: 17 };
    /// store.upsert(&key, &value, 1);
    /// store.rmw(&key, &modification, 1);
    /// let (status, recv): (u8, Receiver<MyU64>) = store.read(&key, 1);
    /// assert!(status == status::OK);
    /// assert_eq!(recv.recv().unwrap().value, value.value + modification.value);
    fn rmw(&self, modification: Self) -> Self;
}
