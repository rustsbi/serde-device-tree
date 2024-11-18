use super::{PropCursor, RefDtb, StructureBlock, ValueCursor, BLOCK_LEN};
use core::{fmt::Debug, ops::Range};
use serde::Deserialize;

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
    pub address_cells: u32,
    pub size_cells: u32,
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
        let len = BLOCK_LEN * (self.config.address_cells + self.config.size_cells) as usize;
        if self.data.len() >= len {
            let mut block = self.data.as_ptr() as *const StructureBlock;
            self.data = &self.data[len..];
            let mut base = 0;
            let mut len = 0;
            for _ in 0..self.config.address_cells {
                unsafe {
                    base = (base << 32) | (*block).as_usize();
                    block = block.offset(1);
                }
            }
            for _ in 0..self.config.size_cells {
                unsafe {
                    len = (len << 32) | (*block).as_usize();
                    block = block.offset(1);
                }
            }
            Some(RegRegion(base..base + len))
        } else {
            None
        }
    }
}
