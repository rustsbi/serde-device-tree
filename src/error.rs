// Copyright (c) 2021 HUST IoT Security Lab
// serde_device_tree is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! When serializing or deserializing device tree goes wrong.

#[cfg(feature = "alloc")]
use alloc::{format, string::String};
use core::fmt;

use crate::common::ALIGN;

/// Represents all possible errors that can occur when serializing or deserializing device tree data.
#[derive(Clone, Debug)]
pub enum Error {
    Typed {
        error_type: ErrorType,
        file_index: usize,
    },
    #[cfg(feature = "alloc")]
    Custom(String),
    #[cfg(not(feature = "alloc"))]
    Custom,
}

/// All error types that would occur from device tree serializing and deserializing.
#[derive(Debug, Clone, Copy)]
pub enum ErrorType {
    Unaligned {
        ptr_value: usize,
        align: usize,
    },
    InvalidMagic {
        wrong_magic: u32,
    },
    IncompatibleVersion {
        last_comp_version: u32,
        library_supported_version: u32,
    },
    HeaderTooShort {
        header_length: u32,
        at_least_length: u32,
    },
    StructureIndex {
        current_index: u32,
        bound_index: u32,
        structure_or_string: bool,
        overflow_or_underflow: bool,
    },
    U32IndexSpace {
        current_index: u32,
    },
    StringEofUnexpected,
    SliceEofUnexpected {
        expected_length: u32,
        remaining_length: u32,
    },
    TableStringOffset {
        given_offset: u32,
        bound_offset: u32,
    },
    TagEofUnexpected {
        current_index: u32,
        bound_index: u32,
    },
    InvalidTagId {
        wrong_id: u32,
    },
    ExpectStructBegin,
    ExpectStructEnd,
    NoRemainingTags,
    InvalidSerdeTypeLength {
        expected_length: u8,
    },
    DeserializeNotComplete,
    Utf8(core::str::Utf8Error),
}

impl Error {
    #[inline]
    pub const fn unaligned(ptr_value: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::Unaligned {
                ptr_value,
                align: ALIGN,
            },
            file_index: 0,
        }
    }
    #[inline]
    pub const fn invalid_magic(wrong_magic: u32) -> Error {
        Error::Typed {
            error_type: ErrorType::InvalidMagic { wrong_magic },
            file_index: 0,
        }
    }
    #[inline]
    pub const fn incompatible_version(
        last_comp_version: u32,
        library_supported_version: u32,
        file_index: usize,
    ) -> Error {
        Error::Typed {
            error_type: ErrorType::IncompatibleVersion {
                last_comp_version,
                library_supported_version,
            },
            file_index,
        }
    }
    #[inline]
    pub fn header_too_short(header_length: u32, at_least_length: u32, file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::HeaderTooShort {
                header_length,
                at_least_length,
            },
            file_index,
        }
    }
    #[inline]
    pub fn u32_index_space_overflow(current_index: u32, file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::U32IndexSpace { current_index },
            file_index,
        }
    }
    #[inline]
    pub fn structure_index_underflow(
        begin_index: u32,
        at_least_index: u32,
        file_index: usize,
    ) -> Error {
        Error::Typed {
            error_type: ErrorType::StructureIndex {
                current_index: begin_index,
                bound_index: at_least_index,
                structure_or_string: true,
                overflow_or_underflow: false,
            },
            file_index,
        }
    }
    #[inline]
    pub fn structure_index_overflow(
        end_index: u32,
        at_most_index: u32,
        file_index: usize,
    ) -> Error {
        Error::Typed {
            error_type: ErrorType::StructureIndex {
                current_index: end_index,
                bound_index: at_most_index,
                structure_or_string: true,
                overflow_or_underflow: true,
            },
            file_index,
        }
    }
    #[inline]
    pub fn string_index_underflow(
        begin_index: u32,
        at_least_index: u32,
        file_index: usize,
    ) -> Error {
        Error::Typed {
            error_type: ErrorType::StructureIndex {
                current_index: begin_index,
                bound_index: at_least_index,
                structure_or_string: false,
                overflow_or_underflow: false,
            },
            file_index,
        }
    }
    #[inline]
    pub fn string_index_overflow(end_index: u32, at_most_index: u32, file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::StructureIndex {
                current_index: end_index,
                bound_index: at_most_index,
                structure_or_string: false,
                overflow_or_underflow: true,
            },
            file_index,
        }
    }
    #[inline]
    pub fn mem_rsvmap_index_underflow(
        begin_index: u32,
        at_least_index: u32,
        file_index: usize,
    ) -> Error {
        Error::Typed {
            error_type: ErrorType::StructureIndex {
                current_index: begin_index,
                bound_index: at_least_index,
                structure_or_string: false,
                overflow_or_underflow: false,
            },
            file_index,
        }
    }
    #[inline]
    pub fn string_eof_unpexpected(file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::StringEofUnexpected,
            file_index,
        }
    }
    #[inline]
    pub fn slice_eof_unpexpected(
        expected_length: u32,
        remaining_length: u32,
        file_index: usize,
    ) -> Error {
        Error::Typed {
            error_type: ErrorType::SliceEofUnexpected {
                expected_length,
                remaining_length,
            },
            file_index,
        }
    }
    #[inline]
    pub fn table_string_offset(given_offset: u32, bound_offset: u32, file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::TableStringOffset {
                given_offset,
                bound_offset,
            },
            file_index,
        }
    }
    #[inline]
    pub fn tag_eof_unexpected(current_index: u32, bound_index: u32, file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::TagEofUnexpected {
                current_index,
                bound_index,
            },
            file_index,
        }
    }
    #[inline]
    pub fn invalid_tag_id(wrong_id: u32, file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::InvalidTagId { wrong_id },
            file_index,
        }
    }
    #[inline]
    pub fn invalid_serde_type_length(expected_length: u8, file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::InvalidSerdeTypeLength { expected_length },
            file_index,
        }
    }
    #[inline]
    pub fn deserialize_not_complete(file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::DeserializeNotComplete,
            file_index,
        }
    }
    #[inline]
    pub fn utf8(error: core::str::Utf8Error, file_index: usize) -> Error {
        Error::Typed {
            error_type: ErrorType::Utf8(error),
            file_index,
        }
    }
    #[inline]
    pub fn expected_struct_begin() -> Error {
        Error::Typed {
            error_type: ErrorType::ExpectStructBegin,
            file_index: 0,
        }
    }
    #[inline]
    pub fn expected_struct_end() -> Error {
        Error::Typed {
            error_type: ErrorType::ExpectStructEnd,
            file_index: 0,
        }
    }
    #[inline]
    pub fn no_remaining_tags() -> Error {
        Error::Typed {
            error_type: ErrorType::NoRemainingTags,
            file_index: 0,
        }
    }
}

/// Alias for a Result with the error type `serde_device_tree:::Error`.
pub type Result<T> = core::result::Result<T, Error>;

impl serde::de::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: fmt::Display,
    {
        #[cfg(feature = "alloc")]
        {
            Self::Custom(format!("{}", _msg))
        }

        #[cfg(not(feature = "alloc"))]
        {
            Self::Custom
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Typed {
                error_type: ErrorType::InvalidMagic { wrong_magic },
                file_index,
            } => write!(
                f,
                "Error(invalid magic, value: {}, index: {})",
                wrong_magic, file_index
            ),
            // todo: format other error types
            others => write!(f, "{:?}", others),
        }
    }
}
