#![feature(ptr_metadata)]
// #![no_std]

extern crate alloc;

pub mod de;
pub mod error;

pub use de::from_raw;
