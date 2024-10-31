//! Deserialize device tree data to a Rust data structure,
//! the memory region contains dtb file should be mutable.

use crate::error::Error as DtError;
use serde::de;

mod cursor;
mod data;
// mod group;
mod node;
mod node_seq;
mod reg;
mod str_seq;
// mod r#struct;
mod structs;

const VALUE_DESERIALIZER_NAME: &str = "$serde_device_tree$de_mut$ValueDeserializer";

pub use structs::{Dtb, DtbPtr};
pub mod buildin {
    pub use super::{node::Node, node_seq::NodeSeq, reg::Reg, str_seq::StrSeq};
}

use cursor::{BodyCursor, Cursor, PropCursor};
use data::{ValueCursor, ValueDeserializer};
use reg::RegConfig;
use structs::{RefDtb, StructureBlock, BLOCK_LEN};

/// 从 [`RefDtb`] 反序列化一个描述设备树的 `T` 类型实例。
///
/// 这个函数在没有堆的环境中执行，
/// 因此可以在操作系统启动的极早期或无动态分配的嵌入式系统中使用。
pub fn from_raw_mut<'de, T>(dtb: RefDtb<'de>) -> Result<T, DtError>
where
    T: de::Deserialize<'de>,
{
    // 根节点的名字固定为空字符串，
    // 从一个跳过根节点名字的光标初始化解析器。
    let mut d = ValueDeserializer {
        dtb,
        reg: RegConfig::DEFAULT,
        cursor: ValueCursor::Body(BodyCursor::ROOT),
    };
    T::deserialize(&mut d).and_then(|t| {
        // 解析必须完成
        if d.is_complete_on() {
            Ok(t)
        } else {
            Err(DtError::deserialize_not_complete(d.file_index_on()))
        }
    })
}

// For map type, we should send root item to trans dtb and reg
enum StructAccessType<'de> {
    Map(bool),
    Seq(&'de str),
    Struct(&'static [&'static str]),
}

/// 结构体解析状态。
struct StructAccess<'de, 'b> {
    access_type: StructAccessType<'de>,
    temp: Temp,
    de: &'b mut ValueDeserializer<'de>,
}

/// 用于跨键-值传递的临时变量。
///
/// 解析键（名字）时将确定值类型，保存 `Temp` 类型的状态。
/// 根据状态分发值解析器。
enum Temp {
    Node(BodyCursor, BodyCursor),
    Group(BodyCursor),
    Prop(BodyCursor, PropCursor),
}

impl<'de> de::MapAccess<'de> for StructAccess<'de, '_> {
    type Error = DtError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if let StructAccessType::Map(flag) = self.access_type {
            if !flag {
                return seed
                    .deserialize(de::value::BorrowedStrDeserializer::new("/"))
                    .map(Some);
            }
        }
        let check_contains = |name: &str| -> bool {
            match self.access_type {
                StructAccessType::Struct(fields) => fields.contains(&name),
                _ => true,
            }
        };
        let name = loop {
            let origin_cursor = match self.de.cursor {
                ValueCursor::Body(cursor) => cursor,
                _ => unreachable!("map access's cursor should always be body cursor"),
            };
            match self.de.move_on() {
                // 子节点名字
                Cursor::Title(c) => {
                    let (name, _) = c.split_on(self.de.dtb);

                    let pre_len = name.as_bytes().iter().take_while(|b| **b != b'@').count();
                    // 子节点名字不带 @ 或正在解析 Node 类型
                    if pre_len == name.as_bytes().len() || check_contains(name) {
                        let (node, next) = c.take_node_on(self.de.dtb, name);
                        self.de.cursor = ValueCursor::Body(next);
                        if check_contains(name) {
                            self.temp = Temp::Node(origin_cursor, node);
                            break name;
                        }
                    }
                    // @ 之前的部分是真正的名字，用这个名字搜索连续的一组
                    else {
                        let name_bytes = &name.as_bytes()[..pre_len];
                        let name = unsafe { core::str::from_utf8_unchecked(name_bytes) };
                        let (group, _, next) = c.take_group_on(self.de.dtb, name);
                        self.de.cursor = ValueCursor::Body(next);
                        if check_contains(name) {
                            self.temp = Temp::Group(group);
                            break name;
                        }
                    }
                }
                // 属性条目
                Cursor::Prop(c) => {
                    let (name, next) = c.name_on(self.de.dtb);
                    self.de.cursor = ValueCursor::Body(next);
                    match name {
                        "#address-cells" => {
                            self.de.reg.address_cells = c.map_u32_on(self.de.dtb)?;
                        }
                        "#size-cells" => {
                            self.de.reg.size_cells = c.map_u32_on(self.de.dtb)?;
                        }
                        _ => {}
                    }
                    if check_contains(name) {
                        self.temp = Temp::Prop(origin_cursor, c);
                        break name;
                    }
                }
                // 截止符，结构体解析完成
                Cursor::End => {
                    self.de.step_n(1);
                    return Ok(None);
                }
            }
        };
        seed.deserialize(de::value::BorrowedStrDeserializer::new(name))
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        if let StructAccessType::Map(ref mut flag) = self.access_type {
            if !*flag {
                *flag = true;
                return seed.deserialize(&mut ValueDeserializer {
                    dtb: self.de.dtb,
                    reg: self.de.reg,
                    cursor: self.de.cursor,
                });
            }
        }
        match self.temp {
            Temp::Node(cursor, node_cursor) => {
                // 键是独立节点名字，递归
                match self.access_type {
                    StructAccessType::Map(_) => seed.deserialize(&mut ValueDeserializer {
                        dtb: self.de.dtb,
                        reg: self.de.reg,
                        cursor: ValueCursor::Body(cursor),
                    }),
                    StructAccessType::Struct(_) => seed.deserialize(&mut ValueDeserializer {
                        dtb: self.de.dtb,
                        reg: self.de.reg,
                        cursor: ValueCursor::Body(node_cursor),
                    }),
                    _ => unreachable!(),
                }
            }
            Temp::Group(cursor) => {
                // 键是组名字，构造组反序列化器
                seed.deserialize(&mut ValueDeserializer {
                    dtb: self.de.dtb,
                    reg: self.de.reg,
                    cursor: ValueCursor::Body(cursor),
                })
            }
            Temp::Prop(origin_cursor, cursor) => {
                // 键是属性名字，构造属性反序列化器
                seed.deserialize(&mut ValueDeserializer {
                    dtb: self.de.dtb,
                    reg: self.de.reg,
                    cursor: ValueCursor::Prop(origin_cursor, cursor),
                })
            }
        }
    }
}

impl<'de> de::SeqAccess<'de> for StructAccess<'de, '_> {
    type Error = DtError;

    fn next_element_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if let StructAccessType::Seq(pre_name) = self.access_type {
            match self.de.move_on() {
                // 子节点名字
                Cursor::Title(c) => {
                    let (name, _) = c.split_on(self.de.dtb);
                    let (_, next) = c.take_node_on(self.de.dtb, name);
                    let prev_cursor = match self.de.cursor {
                        ValueCursor::Body(cursor) => cursor,
                        _ => unreachable!(),
                    };

                    let pre_len = name.as_bytes().iter().take_while(|b| **b != b'@').count();
                    let name_bytes = &name.as_bytes()[..pre_len];
                    let name = unsafe { core::str::from_utf8_unchecked(name_bytes) };
                    if pre_name != name {
                        return Ok(None);
                    }
                    self.de.cursor = ValueCursor::Body(next);
                    seed.deserialize(&mut ValueDeserializer {
                        dtb: self.de.dtb,
                        reg: self.de.reg,
                        cursor: ValueCursor::Body(prev_cursor),
                    })
                    .map(Some)
                }
                _ => Ok(None),
            }
        } else {
            unreachable!("SeqAccess should only be accessed by seq");
        }
    }
}
