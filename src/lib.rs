extern crate libc;
extern crate libfaster_sys as ffi;

mod builder;
mod faster_callbacks;
mod faster_error;
pub mod status;
mod util;

pub use crate::builder::FasterKvBuilder;
use crate::faster_callbacks::{read_callback, rmw_callback};
pub use crate::faster_error::FasterError;
use crate::util::*;

use libc::c_void;
use std::ffi::CStr;
use std::ffi::CString;
use std::fs;
use std::sync::mpsc::{channel, Receiver, Sender};

pub struct FasterIteratorRecord {
    pub key: Option<Vec<u8>>,
    pub value: Option<Vec<u8>>,
    result: *mut ffi::faster_iterator_result,
}

impl Drop for FasterIteratorRecord {
    fn drop(&mut self) {
        unsafe {
            ffi::faster_iterator_result_destroy(self.result);
        }
    }
}

pub struct FasterIterator {
    iterator: *mut c_void,
    record: *mut c_void,
}

impl Drop for FasterIterator {
    fn drop(&mut self) {
        unsafe {
            ffi::faster_scan_in_memory_destroy(self.iterator);
            ffi::faster_scan_in_memory_record_destroy(self.record);
        }
    }
}

impl FasterIterator {
    pub fn get_next(&self) -> Option<FasterIteratorRecord> {
        let result = unsafe { ffi::faster_iterator_get_next(self.iterator, self.record) };
        let status = unsafe { (*result).status };
        if !status {
            unsafe { ffi::faster_iterator_result_destroy(result) };
            return None;
        }
        let mut key_vec = Vec::with_capacity(unsafe{(*result).key_length as usize});
        key_vec.copy_from_slice( unsafe { std::slice::from_raw_parts((*result).key, (*result).key_length as usize) });
        let key = Some(key_vec);
        let mut value_vec = Vec::with_capacity(unsafe{(*result).value_length as usize});
        value_vec.copy_from_slice( unsafe { std::slice::from_raw_parts((*result).value, (*result).value_length as usize) });
        let value = Some(value_vec);
        Some(FasterIteratorRecord { key, value, result })
    }
}

#[no_mangle]
pub unsafe extern "C" fn deallocate_vec(vec: *mut u8, length: u64) {
    drop(Vec::from_raw_parts(vec, length as usize, length as usize));
}

pub struct FasterKv {
    faster_t: *mut ffi::faster_t,
    storage_dir: Option<String>,
}

impl FasterKv {
    pub fn upsert<K, V>(&self, key: &K, value: &V, monotonic_serial_number: u64) -> u8
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let mut cloned_key = Vec::with_capacity(key.as_ref().len());
        cloned_key.extend_from_slice(key.as_ref());
        let cloned_key_length = cloned_key.len();
        let cloned_key_ptr = cloned_key.as_ptr();
        let mut cloned_value = Vec::with_capacity(value.as_ref().len());
        cloned_value.extend_from_slice(value.as_ref());
        let cloned_value_length = cloned_value.len();
        let cloned_value_ptr = cloned_value.as_mut_ptr();
        std::mem::forget(cloned_key);
        std::mem::forget(cloned_value);
        unsafe {
            ffi::faster_upsert(
                self.faster_t,
                cloned_key_ptr,
                cloned_key_length as u64,
                cloned_value_ptr,
                cloned_value_length as u64,
                monotonic_serial_number,
            )
        }
    }

    pub fn read<K>(&self, key: K, monotonic_serial_number: u64) -> (u8, Receiver<Vec<u8>>)
    where
        K: AsRef<[u8]>,
    {
        let mut cloned_key = Vec::with_capacity(key.as_ref().len());
        cloned_key.extend_from_slice(key.as_ref());
        let cloned_key_length = cloned_key.len();
        let cloned_key_ptr = cloned_key.as_ptr();
        let (sender, receiver) = channel();
        let sender_ptr: *mut Sender<Vec<u8>> = Box::into_raw(Box::new(sender));
        std::mem::forget(cloned_key);
        let status = unsafe {
            ffi::faster_read(
                self.faster_t,
                cloned_key_ptr,
                cloned_key_length as u64,
                monotonic_serial_number,
                Some(read_callback),
                sender_ptr as *mut libc::c_void,
            )
        };
        (status, receiver)
    }

    pub fn rmw<K, V>(
        &self,
        key: K,
        value: V,
        rmw_logic: fn(current: &[u8], modification: &[u8]) -> Vec<u8>,
        monotonic_serial_number: u64,
    ) -> u8
    where
        K: AsRef<[u8]>,
        V: AsRef<[u8]>,
    {
        let mut cloned_key = Vec::with_capacity(key.as_ref().len());
        cloned_key.extend_from_slice(key.as_ref());
        let cloned_key_length = cloned_key.len();
        let cloned_key_ptr = cloned_key.as_ptr();
        let mut cloned_value = Vec::with_capacity(value.as_ref().len());
        cloned_value.extend_from_slice(value.as_ref());
        let cloned_value_length = cloned_value.len();
        let cloned_value_ptr = cloned_value.as_ptr();
        std::mem::forget(cloned_key);
        std::mem::forget(cloned_value);
        unsafe {
            ffi::faster_rmw(
                self.faster_t,
                cloned_key_ptr,
                cloned_key_length as u64,
                cloned_value_ptr,
                cloned_value_length as u64,
                monotonic_serial_number,
                Some(rmw_callback),
                &rmw_logic as *const _ as *mut c_void,
            )
        }
    }

    pub fn delete<K>(&self, key: K, monotonic_serial_number: u64) -> u8
    where
        K: AsRef<[u8]>,
    {
        let mut cloned_key = Vec::with_capacity(key.as_ref().len());
        cloned_key.extend_from_slice(key.as_ref());
        let cloned_key_length = cloned_key.len();
        let cloned_key_ptr = cloned_key.as_ptr();
        std::mem::forget(cloned_key);
        unsafe {
            ffi::faster_delete(
                self.faster_t,
                cloned_key_ptr,
                cloned_key_length as u64,
                monotonic_serial_number,
            )
        }
    }

    pub fn get_iterator(&self) -> FasterIterator {
        let iterator = unsafe { ffi::faster_scan_in_memory_init(self.faster_t) };
        let record = unsafe { ffi::faster_scan_in_memory_record_init() };
        FasterIterator { iterator, record }
    }

    pub fn size(&self) -> u64 {
        unsafe { ffi::faster_size(self.faster_t) }
    }

    pub fn checkpoint(&self) -> Result<CheckPoint, FasterError> {
        if self.storage_dir.is_none() {
            return Err(FasterError::InvalidType);
        }

        let result = unsafe { ffi::faster_checkpoint(self.faster_t) };
        match result.is_null() {
            true => Err(FasterError::CheckpointError),
            false => {
                let boxed = unsafe { Box::from_raw(result) }; // makes sure memory is dropped
                let token_str =
                    unsafe { CStr::from_ptr((*boxed).token).to_str().unwrap().to_owned() };

                let checkpoint = CheckPoint {
                    checked: (*boxed).checked,
                    token: token_str,
                };
                Ok(checkpoint)
            }
        }
    }

    pub fn checkpoint_index(&self) -> Result<CheckPoint, FasterError> {
        if self.storage_dir.is_none() {
            return Err(FasterError::InvalidType);
        }

        let result = unsafe { ffi::faster_checkpoint(self.faster_t) };
        match result.is_null() {
            true => Err(FasterError::CheckpointError),
            false => {
                let boxed = unsafe { Box::from_raw(result) }; // makes sure memory is dropped
                let token_str =
                    unsafe { CStr::from_ptr((*boxed).token).to_str().unwrap().to_owned() };

                let checkpoint = CheckPoint {
                    checked: (*boxed).checked,
                    token: token_str,
                };
                Ok(checkpoint)
            }
        }
    }

    pub fn checkpoint_hybrid_log(&self) -> Result<CheckPoint, FasterError> {
        if self.storage_dir.is_none() {
            return Err(FasterError::InvalidType);
        }

        let result = unsafe { ffi::faster_checkpoint(self.faster_t) };
        match result.is_null() {
            true => Err(FasterError::CheckpointError),
            false => {
                let boxed = unsafe { Box::from_raw(result) }; // makes sure memory is dropped
                let token_str =
                    unsafe { CStr::from_ptr((*boxed).token).to_str().unwrap().to_owned() };

                let checkpoint = CheckPoint {
                    checked: (*boxed).checked,
                    token: token_str,
                };
                Ok(checkpoint)
            }
        }
    }

    pub fn recover(
        &self,
        index_token: String,
        hybrid_log_token: String,
    ) -> Result<Recover, FasterError> {
        if self.storage_dir.is_none() {
            return Err(FasterError::InvalidType);
        }
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

        match recover_result.is_null() {
            true => Err(FasterError::RecoveryError),
            false => {
                let boxed = unsafe { Box::from_raw(recover_result) }; // makes sure mem is freed
                let sessions_count = (*boxed).session_ids_count;
                let mut session_ids_vec: Vec<String> = Vec::new();
                for i in 0..sessions_count {
                    let id = unsafe {
                        CStr::from_ptr(((*boxed).session_ids).offset(37 * i as isize))
                            .to_str()
                            .unwrap()
                            .to_owned()
                    };
                    session_ids_vec.push(id);
                }
                let recover = Recover {
                    status: (*boxed).status,
                    version: (*boxed).version,
                    session_ids: session_ids_vec,
                };
                Ok(recover)
            }
        }
    }

    pub fn complete_pending(&self, b: bool) -> () {
        unsafe { ffi::faster_complete_pending(self.faster_t, b) }
    }

    pub fn start_session(&self) -> String {
        unsafe {
            let c_guid = ffi::faster_start_session(self.faster_t);
            let rust_str = CStr::from_ptr(c_guid).to_str().unwrap().to_owned();
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
        unsafe { ffi::faster_stop_session(self.faster_t) }
    }

    pub fn refresh(&self) -> () {
        unsafe {
            ffi::faster_refresh_session(self.faster_t);
        }
    }

    pub fn dump_distribution(&self) -> () {
        unsafe {
            ffi::faster_dump_distribution(self.faster_t);
        }
    }

    pub fn grow_index(&self) -> bool {
        unsafe { ffi::faster_grow_index(self.faster_t) }
    }

    // Warning: Calling this will remove the stored data
    pub fn clean_storage(&self) -> Result<(), FasterError> {
        match &self.storage_dir {
            None => Err(FasterError::InvalidType),
            Some(dir) => {
                fs::remove_dir_all(dir)?;
                Ok(())
            }
        }
    }

    fn destroy(&self) -> () {
        unsafe {
            ffi::faster_destroy(self.faster_t);
        }
    }
}

impl Default for FasterKv {
    fn default() -> Self {
        FasterKvBuilder::new(1 << 15, 1024 * 1024 * 1024)
            .build()
            .unwrap()
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
