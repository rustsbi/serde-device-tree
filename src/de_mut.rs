//! 将设备树解析为 Rust 数据类型。
//!
//! 将破坏性地利用设备树空间以完全避免内存分配。

use crate::Error;
use serde::{
    de::{self, SeqAccess},
    Deserialize,
};

mod device_tree;
mod node_seq;
mod str_seq;

pub use node_seq::NodeSeq;
pub use str_seq::StrSeq;

use device_tree::DeviceTree;

const U32_LEN: usize = core::mem::size_of::<u32>();
const OF_DT_END_STR: u8 = 0;
const OF_DT_BEGIN_NODE: [u8; 4] = [0, 0, 0, 1];
const OF_DT_END_NODE: [u8; 4] = [0, 0, 0, 2];
const OF_DT_PROP: [u8; 4] = [0, 0, 0, 3];
const OF_DT_NOP: [u8; 4] = [0, 0, 0, 4];

/// 从指向设备树的指针解析设备树并构造目标类型。
///
/// # Safety
///
/// TODO
pub unsafe fn from_raw_mut<'de, T>(ptr: *mut u8) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    let mut d = Deserializer {
        expect_strings: false,
        loaded: Tag::End,
        device_tree: DeviceTree::from_raw_ptr(ptr)?,
    };
    T::deserialize(&mut d)
}

struct Deserializer {
    expect_strings: bool,
    loaded: Tag,
    device_tree: DeviceTree,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
struct StructureBlock([u8; U32_LEN]);

impl From<u32> for StructureBlock {
    fn from(val: u32) -> Self {
        Self(u32::to_ne_bytes(val))
    }
}

#[derive(Debug)]
enum Tag {
    Begin(&'static str),
    MultipleBlock(&'static str, &'static [u8]),
    Prop(&'static str, &'static [u8]),
    End,
}

impl<'de, 'b> de::Deserializer<'de> for &'b mut Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("any")
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_bool(true)
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("i8")
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("i16")
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("i32")
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("i64")
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("u8")
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("u16")
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.loaded {
            Tag::Prop(_, bytes) => match *bytes {
                [a, b, c, d] => visitor.visit_u32(u32::from_be_bytes([a, b, c, d])),
                _ => todo!(),
            },
            Tag::Begin(_) | Tag::MultipleBlock(_, _) | Tag::End => todo!(),
        }
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("u64")
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("f32")
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("f64")
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("char")
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.loaded {
            Tag::Prop(_, bytes) => {
                visitor.visit_borrowed_str(unsafe { core::str::from_utf8_unchecked(bytes) })
            }
            Tag::Begin(_) | Tag::MultipleBlock(_, _) | Tag::End => todo!(),
        }
    }

    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("string")
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.loaded {
            Tag::MultipleBlock(_, block) => {
                if self.expect_strings {
                    self.expect_strings = false;
                    visitor.visit_borrowed_bytes(self.device_tree.strings)
                } else {
                    visitor.visit_borrowed_bytes(block)
                }
            }
            Tag::Begin(_) | Tag::Prop(_, _) | Tag::End => todo!(),
        }
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("byte_buf")
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("unit")
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("unit_struct")
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("new_type_struct")
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("seq")
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("tuple")
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if name == node_seq::NODE_INNER_IDENT && len == 2 {
            visitor.visit_seq(NodeAccess::new(self))
        } else {
            todo!("tuple_struct")
        }
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("map")
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(StructVisitor { fields, de: self })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("enum")
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("identifier")
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("ignored_any")
    }
}

struct StructVisitor<'a> {
    fields: &'static [&'static str],
    de: &'a mut Deserializer,
}

impl<'de, 'b> de::MapAccess<'de> for StructVisitor<'b> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let name = loop {
            self.de.loaded = self.de.device_tree.next()?;
            match self.de.loaded {
                Tag::Begin(name) => {
                    if self.fields.contains(&name) {
                        break name;
                    }
                    self.de.device_tree.skip_node()?;
                }
                Tag::MultipleBlock(name, _) => {
                    if self.fields.contains(&name) {
                        break name;
                    }
                }
                Tag::Prop(name, _) => {
                    if self.fields.contains(&name) {
                        break name;
                    }
                }
                Tag::End => return Ok(None),
            }
        };
        seed.deserialize(de::value::BorrowedStrDeserializer::new(name))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.de.loaded {
            Tag::Begin(_name) | Tag::MultipleBlock(_name, _) | Tag::Prop(_name, _) => {
                seed.deserialize(&mut *self.de)
            }
            Tag::End => todo!(),
        }
    }
}

struct NodeAccess<'a>(&'a mut Deserializer);

impl<'a> NodeAccess<'a> {
    fn new(de: &'a mut Deserializer) -> Self {
        de.expect_strings = true;
        Self(de)
    }
}

impl<'de, 'b> SeqAccess<'de> for NodeAccess<'b> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.0).map(Some)
    }
}
