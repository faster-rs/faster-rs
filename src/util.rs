extern crate libc;
extern crate libfaster_sys as ffi;

pub struct CheckPoint {
    pub checked: bool,
    pub token: String,
}

pub struct Recover {
    pub status: u8,
    pub version: u32,
    pub session_ids: Vec<String>,
}
