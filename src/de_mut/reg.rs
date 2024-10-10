use super::{PropCursor, RefDtb, StructureBlock, BLOCK_LEN};
use core::{fmt::Debug, marker::PhantomData, mem::MaybeUninit, ops::Range};
use serde::{de, Deserialize};

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

impl<'de, 'b> Deserialize<'de> for Reg<'b> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de, 'b> {
            marker: PhantomData<Reg<'b>>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de, 'b> de::Visitor<'de> for Visitor<'de, 'b> {
            type Value = Reg<'b>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "struct Reg")
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // 结构体转为内存切片，然后拷贝过来
                if v.len() == core::mem::size_of::<Self::Value>() {
                    Ok(Self::Value::from_raw_parts(v.as_ptr()))
                } else {
                    Err(E::invalid_length(
                        v.len(),
                        &"`Reg` is copied with wrong size.",
                    ))
                }
            }
        }

        serde::Deserializer::deserialize_newtype_struct(
            deserializer,
            "Reg",
            Visitor {
                marker: PhantomData,
                lifetime: PhantomData,
            },
        )
    }
}

impl Reg<'_> {
    fn from_raw_parts(ptr: *const u8) -> Self {
        // 直接从指针拷贝
        unsafe {
            let mut res = MaybeUninit::<Self>::uninit();
            core::ptr::copy_nonoverlapping(
                ptr,
                res.as_mut_ptr() as *mut _,
                core::mem::size_of::<Self>(),
            );
            res.assume_init()
        }
    }

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
