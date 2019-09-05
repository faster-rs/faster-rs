extern crate libc;
extern crate libfaster_sys as ffi;

use crate::status;

use std::os::raw::c_void;
use std::sync::mpsc::Sender;

#[inline(always)]
pub unsafe extern "C" fn read_callback(
    sender: *mut libc::c_void,
    value: *const u8,
    length: u64,
    status: u32,
) {
    let boxed_sender = Box::from_raw(sender as *mut Sender<Vec<u8>>);
    let sender = *boxed_sender;
    if status == status::OK.into() {
        let mut vec = Vec::with_capacity(length as usize);
        let slice = std::slice::from_raw_parts(value, length as usize);
        vec.extend_from_slice(slice);
        // TODO: log error
        let _ = sender.send(vec);
    }
}

#[inline(always)]
pub unsafe extern "C" fn rmw_callback(
    current: *const u8,
    length_current: u64,
    modification: *const u8,
    length_modification: u64,
    rmw_logic: *mut c_void,
    dst: *mut u8,
) -> u64 {
    let val = std::slice::from_raw_parts(current as *mut u8, length_current as usize);
    let modif = std::slice::from_raw_parts(modification as *mut u8, length_modification as usize);
    let cb = rmw_logic as *mut Option<fn(&[u8], &[u8]) -> Vec<u8>>;
    let cb = (*cb).unwrap();
    let modified = cb(&val, &modif);
    if dst != std::ptr::null_mut() {
        modified.as_ptr().copy_to(dst, modified.len());
    }
    modified.len() as u64
}
