use crate::faster_value::FasterValue;
use crate::FasterKey;
use serde::{Deserialize, Serialize};
use std::ops::Add;

impl<T> FasterKey for T where T: Serialize + Deserialize<'static> {}

macro_rules! primitive_impl {
    ($ty:ident, $method:ident $($cast:tt)*) => {
        impl FasterValue for $ty {
            #[inline]
            fn rmw(&self, modification: Self) -> Self {
                $method(*self, modification)
            }
        }
    };
}

macro_rules! owned_impl {
    ($ty:ident, $method:ident $($cast:tt)*) => {
        impl FasterValue for $ty {
            #[inline]
            fn rmw(&self, modification: Self) -> Self {
                $method(self, modification)
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
primitive_impl!(usize, rmw_add);
primitive_impl!(u8, rmw_add);
primitive_impl!(u16, rmw_add);
primitive_impl!(u32, rmw_add);
primitive_impl!(u64, rmw_add);
primitive_impl!(f32, rmw_add);
primitive_impl!(f64, rmw_add);

fn rmw_char(_old: char, new: char) -> char {
    new
}
primitive_impl!(char, rmw_char);

fn rmw_string(old: &String, new: String) -> String {
    let mut new_string = old.clone();
    new_string.push_str(new.as_str());
    new_string
}
owned_impl!(String, rmw_string);

impl<T: Clone + Serialize + Deserialize<'static>> FasterValue for Vec<T> {
    #[inline]
    fn rmw(&self, new: Vec<T>) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len() + new.len());
        for e in self {
            result.push(e.clone());
        }
        for e in new {
            result.push(e.clone());
        }
        result
    }
}
