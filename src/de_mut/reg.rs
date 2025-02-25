use super::{BLOCK_LEN, PropCursor, RefDtb, ValueCursor};
use core::{fmt::Debug, ops::Range};
use serde::{Deserialize, Serialize};

/// 节点地址空间。
pub struct Reg<'de>(Inner<'de>);

pub(super) struct Inner<'de> {
    pub dtb: RefDtb<'de>,
    pub cursor: PropCursor,
    pub reg: RegConfig,
}

/// 地址段迭代器。
pub struct RegIter<'de> {
    data: &'de [u8],
    config: RegConfig,
}

#[derive(Clone, Debug)]
pub struct RegRegion(pub Range<usize>);

/// 节点地址空间格式。
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(super) struct RegConfig {
    pub address_cells: usize,
    pub size_cells: usize,
}

impl RegConfig {
    pub const DEFAULT: Self = Self {
        address_cells: 2,
        size_cells: 1,
    };
}

impl<'de> Deserialize<'de> for Reg<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value_deserialzer = super::ValueDeserializer::deserialize(deserializer)?;

        let inner = Inner {
            dtb: value_deserialzer.dtb,
            reg: value_deserialzer.reg,
            cursor: match value_deserialzer.cursor {
                ValueCursor::Prop(_, cursor) => cursor,
                _ => {
                    unreachable!("Reg Deserialize should only be called by prop cursor")
                }
            },
        };

        Ok(Self(inner))
    }
}

impl Reg<'_> {
    pub fn iter(&self) -> RegIter {
        RegIter {
            data: self.0.cursor.data_on(self.0.dtb),
            config: self.0.reg,
        }
    }
}

impl Debug for Reg<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iter = self.iter();
        if let Some(s) = iter.next() {
            write!(f, "[{:#x?}", s.0)?;
            for s in iter {
                write!(f, ", {:#x?}", s.0)?;
            }
            write!(f, "]")
        } else {
            write!(f, "[]")
        }
    }
}

impl Iterator for RegIter<'_> {
    type Item = RegRegion;

    fn next(&mut self) -> Option<Self::Item> {
        let len = BLOCK_LEN * (self.config.address_cells + self.config.size_cells);
        if self.data.len() >= len {
            let (current_block, data) = self.data.split_at(len);
            self.data = data;
            let mut base = 0;
            let mut len = 0;
            let mut block_id = 0;
            for _ in 0..self.config.address_cells {
                base = (base << 32)
                    | u32::from_be_bytes(
                        current_block[block_id * 4..(block_id + 1) * 4]
                            .try_into()
                            .unwrap(),
                    ) as usize;
                block_id += 1;
            }
            for _ in 0..self.config.size_cells {
                len = (len << 32)
                    | u32::from_be_bytes(
                        current_block[block_id * 4..(block_id + 1) * 4]
                            .try_into()
                            .unwrap(),
                    ) as usize;
                block_id += 1;
            }
            Some(RegRegion(base..base + len))
        } else {
            None
        }
    }
}

impl<'se> Serialize for Reg<'se> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Pass bytes directly for Reg.
        serializer.serialize_bytes(self.0.cursor.data_on(self.0.dtb))
    }
}
