use crate::{FasterKey, FasterRmw, FasterValue};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ops::Add;

impl<T> FasterKey for T where T: Serialize + DeserializeOwned {}
impl<T> FasterValue for T where T: Serialize + DeserializeOwned {}

macro_rules! primitive_impl {
    ($ty:ident, $method:ident $($cast:tt)*) => {
        impl FasterRmw for $ty {
            #[inline]
            fn rmw(&self, modification: Self) -> Self {
                $method(*self, modification)
            }
        }
    };
}

fn rmw_bool(_old: bool, new: bool) -> bool {
    new
}
primitive_impl!(bool, rmw_bool);

fn rmw_add<T: Add<Output = T>>(current: T, modification: T) -> T {
    current + modification
}
primitive_impl!(isize, rmw_add);
primitive_impl!(i8, rmw_add);
primitive_impl!(i16, rmw_add);
primitive_impl!(i32, rmw_add);
primitive_impl!(i64, rmw_add);
primitive_impl!(i128, rmw_add);
primitive_impl!(usize, rmw_add);
primitive_impl!(u8, rmw_add);
primitive_impl!(u16, rmw_add);
primitive_impl!(u32, rmw_add);
primitive_impl!(u64, rmw_add);
primitive_impl!(u128, rmw_add);
primitive_impl!(f32, rmw_add);
primitive_impl!(f64, rmw_add);

fn rmw_char(_old: char, new: char) -> char {
    new
}
primitive_impl!(char, rmw_char);
