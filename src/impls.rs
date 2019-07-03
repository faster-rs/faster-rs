use crate::{FasterKey, FasterRmw, FasterValue};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::Add;

impl<T> FasterKey for T where T: Serialize + DeserializeOwned {}
impl<T> FasterValue for T where T: Serialize + DeserializeOwned {}

impl<T> FasterRmw for T
where
    T: Add<Output = T> + Serialize + DeserializeOwned,
{
    fn rmw(self, modification: Self) -> Self {
        self + modification
    }
}
