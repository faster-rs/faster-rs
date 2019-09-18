use crate::{FasterError, FasterKv};
use std::ffi::CString;

pub struct FasterKvBuilder<'a> {
    table_size: u64,
    log_size: u64,
    storage: Option<&'a str>,
    log_mutable_fraction: f64,
    pre_allocate_log: bool,
}

impl<'a> FasterKvBuilder<'a> {
    pub fn new(table_size: u64, log_size: u64) -> FasterKvBuilder<'a> {
        FasterKvBuilder {
            table_size,
            log_size,
            storage: None,
            log_mutable_fraction: 0.9,
            pre_allocate_log: false,
        }
    }

    pub fn with_disk(&mut self, path: &'a str) -> &mut FasterKvBuilder<'a> {
        self.storage = Some(path);
        self
    }

    pub fn with_log_mutable_fraction(&mut self, fraction: f64) -> &mut FasterKvBuilder<'a> {
        self.log_mutable_fraction = fraction;
        self
    }

    pub fn set_pre_allocate_log(&mut self, pre_allocate_log: bool) -> &mut FasterKvBuilder<'a> {
        self.pre_allocate_log = pre_allocate_log;
        self
    }

    pub fn build(&self) -> Result<FasterKv, FasterError<'static>> {
        if !(self.log_mutable_fraction > 0.0 && self.log_mutable_fraction <= 1.0) {
            return Err(FasterError::BuilderError(
                "Log mutable fraction must be between 0 and 1",
            ));
        }
        unsafe {
            let mut storage_dir = None;
            let faster_t = match self.storage {
                None => ffi::faster_open(self.table_size, self.log_size, self.pre_allocate_log),
                Some(path) => {
                    let storage_str = CString::new(path).unwrap();
                    let ptr_raw = storage_str.into_raw();
                    let ft = ffi::faster_open_with_disk(
                        self.table_size,
                        self.log_size,
                        ptr_raw,
                        self.log_mutable_fraction,
                        self.pre_allocate_log,
                    );
                    storage_dir = CString::from_raw(ptr_raw).into_string().ok();
                    ft
                }
            };
            Ok(FasterKv {
                faster_t,
                storage_dir,
            })
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::FasterKvBuilder;
    use tempfile::TempDir;
    #[test]
    fn can_build_with_disk() {
        let dir = TempDir::new().unwrap();
        let dir_str = dir.path().to_str().unwrap();
        let mut builder = FasterKvBuilder::new(1 << 15, 1024 * 1024 * 1024);
        builder
            .with_disk(dir_str)
            .set_pre_allocate_log(true)
            .with_log_mutable_fraction(0.8);
        let kv = builder.build().unwrap();
        let storage = &kv.storage_dir;
        assert_eq!(storage.as_ref().unwrap(), dir_str);
    }
}
