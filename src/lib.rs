extern crate libc;
extern crate libfaster_sys as ffi;


use std::ffi::CString;
use std::fs;

pub mod status;

pub struct FasterKv {
    faster_t: *mut ffi::faster_t,
    storage_dir: String
}

impl FasterKv {
    pub fn new(table_size: u64, log_size: u64, storage_name: String) -> std::io::Result<FasterKv> {
        let saved_dir = storage_name.clone();
        let storage_str = CString::new(storage_name).unwrap();
        let ptr_raw = storage_str.into_raw();
        unsafe {
            let ft = ffi::faster_open_with_disk(table_size, log_size, ptr_raw);
            let _ = CString::from_raw(ptr_raw); // retake pointer to free mem
            Ok(FasterKv {
                faster_t: ft,
                storage_dir: saved_dir
            })
        }
    }

    pub fn upsert(&self, key: u64, value: u64) -> () {
        unsafe {
            ffi::faster_upsert(self.faster_t, key, value);
        }
    }

    pub fn read(&self, key: u64) -> u8 {
        unsafe {
            ffi::faster_read(self.faster_t, key)
        }
    }

    pub fn rmw(&self, key: u64, value: u64) -> u8 {
        unsafe {
            ffi::faster_rmw(self.faster_t, key, value)
        }
    }

    pub fn size(&self) -> u64 {
        unsafe {
            ffi::faster_size(self.faster_t)
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

// In order to make sure we release the resources the C interface has allocated
impl Drop for FasterKv {
    fn drop(&mut self) {
        self.destroy();
    }
}


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

            store.upsert(key, value);

            let res = store.read(key);
            assert!(res == status::OK);

            let res = store.read(2 as u64);
            assert!(res == status::NOT_FOUND);


            let rmw = store.rmw(key, 5 as u64);
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
}
