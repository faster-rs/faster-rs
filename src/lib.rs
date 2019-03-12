extern crate bincode;
extern crate libc;
extern crate libfaster_sys as ffi;

pub mod status;
pub mod util;
pub mod faster_value;

use crate::util::*;
pub use crate::faster_value::FasterValue;

use serde::{Serialize, Deserialize};
use std::ffi::CStr;
use std::ffi::CString;
use std::fs;
use std::io;
use std::sync::mpsc::{channel, Sender, Receiver};

pub struct FasterKv {
    faster_t: *mut ffi::faster_t,
    storage_dir: String
}

impl FasterKv {
    pub fn new(table_size: u64, log_size: u64, storage_name: String) -> Result<FasterKv, io::Error> {
        let saved_dir = storage_name.clone();
        let storage_str = CString::new(storage_name).unwrap();
        let ptr_raw = storage_str.into_raw();
        let faster_t = unsafe {
            let ft = ffi::faster_open_with_disk(table_size, log_size, ptr_raw);
            let _ = CString::from_raw(ptr_raw); // retake pointer to free mem
            ft
        };
        Ok(FasterKv { faster_t: faster_t, storage_dir: saved_dir })
    }

    pub fn upsert<T: Serialize>(&self, key: u64, value: &T) -> u8 {
        let mut encoded = bincode::serialize(value).unwrap();
        unsafe {
            ffi::faster_upsert(self.faster_t, key, encoded.as_mut_ptr(), encoded.len() as u64)
        }
    }

    pub fn read<'a, T: FasterValue<'a, T> + Deserialize<'a> + Serialize>(&'a self, key: u64) -> (u8, Receiver<T>) {
        let (sender, receiver) = channel();
        let sender_ptr: *mut Sender<T> = Box::into_raw(Box::new(sender));
        let status = unsafe {
            ffi::faster_read(self.faster_t, key, Some(T::read_callback), sender_ptr as *mut libc::c_void)
        };
        (status, receiver)
    }

    pub fn rmw<'a, T: FasterValue<'a, T> + Deserialize<'a> + Serialize>(&'a self, key: u64, value: &T) -> u8 {
        let mut encoded = bincode::serialize(value).unwrap();
        unsafe {
            ffi::faster_rmw(self.faster_t, key, encoded.as_mut_ptr(), encoded.len() as u64, Some(T::rmw_callback))
        }
    }

    pub fn size(&self) -> u64 {
        unsafe {
            ffi::faster_size(self.faster_t)
        }
    }

    pub fn checkpoint(&self) -> Option<CheckPoint> {
        let result = unsafe { ffi::faster_checkpoint(self.faster_t) };
        if result.is_null() {
            None
        } else {
            let boxed = unsafe { Box::from_raw(result) }; // makes sure memory is dropped
            let token_str = unsafe {CStr::from_ptr((*boxed).token)
                .to_str()
                .unwrap()
                .to_owned() };

            let checkpoint = CheckPoint { checked: (*boxed).checked, token: token_str};
            Some(checkpoint)
        }
    }

    pub fn recover(&self, index_token: String, hybrid_log_token: String) -> Option<Recover> {
        let index_token_c = CString::new(index_token).unwrap();
        let index_token_ptr = index_token_c.into_raw();

        let hybrid_token_c = CString::new(hybrid_log_token).unwrap();
        let hybrid_token_ptr = hybrid_token_c.into_raw();

        let recover_result = unsafe {
            let rec = ffi::faster_recover(self.faster_t, index_token_ptr, hybrid_token_ptr);
            let _ = CString::from_raw(index_token_ptr);
            let _ = CString::from_raw(hybrid_token_ptr);
            rec
        };

        if !recover_result.is_null() {
            let boxed = unsafe { Box::from_raw(recover_result) }; // makes sure mem is freed
            let sessions_count = (*boxed).session_ids_count;
            let mut session_ids_vec: Vec<String> = Vec::new();
            for i in 0..sessions_count {
                let id = unsafe {
                    CStr::from_ptr((*(*boxed).session_ids).offset(i as isize))
                        .to_str()
                        .unwrap()
                        .to_owned()
                };
                session_ids_vec.push(id);
            }
            let recover = Recover { status: (*boxed).status, version: (*boxed).version, session_ids: session_ids_vec};
            Some(recover)
        } else {
            None
        }
    }

    pub fn complete_pending(&self, b: bool) -> () {
        unsafe {
            ffi::faster_complete_pending(self.faster_t, b)
        }
    }

    pub fn start_session(&self) -> String {
        unsafe {
            let c_guid = ffi::faster_start_session(self.faster_t);
            let rust_str = CStr::from_ptr(c_guid)
                .to_str()
                .unwrap()
                .to_owned();
            rust_str
        }
    }

    pub fn continue_session(&self, token: String) -> u64 {
        let token_str = CString::new(token).unwrap();
        let token_ptr = token_str.into_raw();
        unsafe {
            let result = ffi::faster_continue_session(self.faster_t, token_ptr);
            let _ = CString::from_raw(token_ptr);
            result
        }
    }

    pub fn stop_session(&self) -> () {
        unsafe {
            ffi::faster_stop_session(self.faster_t)
        }
    }

    pub fn refresh(&self) -> () {
        unsafe {
            ffi::faster_refresh_session(self.faster_t);
        }
    }

    // Warning: Calling this will remove the stored data
    pub fn clean_storage(&self) -> std::io::Result<()> {
        fs::remove_dir_all(&self.storage_dir)?;
        Ok(())
    }

    fn destroy(&self) -> () {
        unsafe {
            ffi::faster_destroy(self.faster_t);
        }
    }
}

// In order to make sure we release the resources the C interface has allocated for the store
impl Drop for FasterKv {
    fn drop(&mut self) {
        self.destroy();
    }
}

unsafe impl Send for FasterKv {}
unsafe impl Sync for FasterKv {}


#[cfg(test)]
mod tests {
    use super::*;
    const TABLE_SIZE: u64  = 1 << 14;
    const LOG_SIZE: u64 = 17179869184;

    #[test]
    fn faster_check() {
        if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("storage")) {
            let key: u64 = 1;
            let value: u64 = 1337;

            let upsert = store.upsert(key, &value);
            assert!((upsert == status::OK || upsert == status::PENDING) == true);

            let rmw = store.rmw(key, &(5 as u64));
            assert!(rmw == status::OK);

            assert!(store.size() > 0);

            match store.clean_storage() {
                Ok(()) => assert!(true),
                Err(_err) => assert!(false)
            }
        } else {
            assert!(false)
        }
    }

    #[test]
    fn faster_read_inserted_value() {
        if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("storage1")) {
            let key: u64 = 1;
            let value: u64 = 1337;

            let upsert = store.upsert(key, &value);
            assert!((upsert == status::OK || upsert == status::PENDING) == true);

            let (res, recv): (u8, Receiver<u64>) = store.read(key);
            assert!(res == status::OK);
            assert!(recv.recv().unwrap() == value);

            match store.clean_storage() {
                Ok(()) => assert!(true),
                Err(_err) => assert!(false)
            }
        }
    }

    #[test]
    fn faster_read_missing_value_recv_error() {
        if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("storage2")) {
            let key: u64 = 1;

            let (res, recv): (u8, Receiver<u64>) = store.read(key);
            assert!(res == status::NOT_FOUND);
            assert!(recv.recv().is_err());

            match store.clean_storage() {
                Ok(()) => assert!(true),
                Err(_err) => assert!(false)
            }
        }
    }

    #[test]
    fn faster_rmw_changes_values() {
        if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("storage3")) {
            let key: u64 = 1;
            let value: u64 = 1337;
            let modification: u64 = 100;

            let upsert = store.upsert(key, &value);
            assert!((upsert == status::OK || upsert == status::PENDING) == true);

            let (res, recv): (u8, Receiver<u64>) = store.read(key);
            assert!(res == status::OK);
            assert!(recv.recv().unwrap() == value);

            let rmw = store.rmw(key, &modification);
            assert!((rmw == status::OK || rmw == status::PENDING) == true);

            let (res, recv): (u8, Receiver<u64>) = store.read(key);
            assert!(res == status::OK);
            assert!(recv.recv().unwrap() == value + modification);

            match store.clean_storage() {
                Ok(()) => assert!(true),
                Err(_err) => assert!(false)
            }
        }
    }

    #[test]
    fn faster_rmw_without_upsert() {
        if let Ok(store) = FasterKv::new(TABLE_SIZE, LOG_SIZE, String::from("storage4")) {
            let key: u64 = 1;
            let modification: u64 = 100;

            let rmw = store.rmw(key, &modification);
            assert!((rmw == status::OK || rmw == status::PENDING) == true);

            let (res, recv): (u8, Receiver<u64>) = store.read(key);
            assert!(res == status::OK);
            assert!(recv.recv().unwrap() == modification);

            match store.clean_storage() {
                Ok(()) => assert!(true),
                Err(_err) => assert!(false)
            }
        }
    }
}
