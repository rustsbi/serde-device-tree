use super::BLOCK_LEN;
use core::{fmt::Debug, marker::PhantomData};
use serde::{de, Deserialize};

/// 节点地址空间。
#[derive(Clone, Copy, Debug)]
pub struct Reg {
    pub base: usize,
    pub len: usize,
}

/// 节点地址空间格式。
#[derive(Clone, Copy, Debug)]
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

impl<'de, 'b> Deserialize<'de> for Reg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de> {
            marker: PhantomData<Reg>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de, 'b> de::Visitor<'de> for Visitor<'de> {
            type Value = Reg;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "struct Reg")
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // 结构体转为内存切片，然后拷贝过来
                if v.len() == core::mem::size_of::<Self::Value>() {
                    Ok(unsafe { *(v.as_ptr() as *const Self::Value) })
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

impl RegConfig {
    pub fn build_from(&self, data: &[u8]) -> Option<Reg> {
        let len = (self.address_cells + self.size_cells) as usize;
        if data.len() == BLOCK_LEN * len {
            let mut u32s = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u32, len) }
                .iter()
                .map(|val| u32::from_be(*val));
            let mut reg = Reg { base: 0, len: 0 };
            for _ in 0..self.address_cells {
                reg.base = (reg.base << 32) | u32s.next().unwrap() as usize;
            }
            for _ in 0..self.size_cells {
                reg.len = (reg.len << 32) | u32s.next().unwrap() as usize;
            }
            Some(reg)
        } else {
            None
        }
    }
}
