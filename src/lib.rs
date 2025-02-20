// Copyright (c) 2021 HUST IoT Security Lab
// serde_device_tree is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! This library contains two device tree blob deserializers,
//! one with no-std support,
//! the other one doesn't even need alloc.

#![feature(ptr_metadata)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod de;
pub mod error;
pub mod ser;
pub mod utils;

mod common;
mod de_mut;
mod tag;
mod value;

pub use value::compatible::Compatible;

#[doc(inline)]
pub use de::from_raw;

#[doc(inline)]
pub use de_mut::{buildin, from_raw_mut, Dtb, DtbPtr};

#[doc(inline)]
pub use error::Result;
