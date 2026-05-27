pub mod algo;
pub mod array;
pub mod context;
pub mod event;
pub mod image;
pub mod stream;
pub mod sys;
mod util;
pub mod warp_map;

pub use util::{VpiError, VpiResult};

#[macro_export]
/// helper for VPI flags
macro_rules! flags {
    ($($flag:ident),* $(,)?) => {
        0u64 $(| ($crate::sys::$flag as u64))*
    };
}
