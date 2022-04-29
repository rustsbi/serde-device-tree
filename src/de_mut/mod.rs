//! Deserialize device tree data to a Rust data structure,
//! the memory region contains dtb file should be mutable.

use crate::Error as DtError;
use serde::de;

mod cursor;
mod data;
mod group;
mod node_seq;
mod str_seq;
mod r#struct;
mod structs;

pub use node_seq::NodeSeq;
pub use str_seq::StrSeq;
pub use structs::{Dtb, DtbPtr};

use cursor::{BodyCursor, Cursor, GroupCursor, PropCursor};
use data::BorrowedValueDeserializer;
use group::GroupDeserializer;
use r#struct::StructDeserializer;
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
    let mut d = StructDeserializer {
        dtb,
        cursor: BodyCursor::ROOT,
    };
    T::deserialize(&mut d).and_then(|t| {
        // 解析必须完成
        if d.cursor.is_complete_on(dtb) {
            Ok(t)
        } else {
            Err(DtError::deserialize_not_complete(
                d.cursor.file_index_on(d.dtb),
            ))
        }
    })
}

/// 结构体解析状态。
struct StructAccess<'de, 'b> {
    fields: &'static [&'static str],
    temp: Temp,
    de: &'b mut StructDeserializer<'de>,
}

/// 用于跨键-值传递的临时变量。
///
/// 解析键（名字）时将确定值类型，保存 `Temp` 类型的状态。
/// 根据状态分发值解析器。
enum Temp {
    Node,
    Group(GroupCursor, usize, usize),
    Prop(PropCursor),
}

impl<'de, 'b> de::MapAccess<'de> for StructAccess<'de, 'b> {
    type Error = DtError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        let name = loop {
            match self.de.move_next() {
                // 子节点名字
                Cursor::Title(c) => {
                    let (name, sub) = c.split_on(self.de.dtb);

                    let pre_len = name.as_bytes().iter().take_while(|b| **b != b'@').count();
                    // 子节点名字不带 @
                    if pre_len == name.as_bytes().len() {
                        self.de.cursor = sub;
                        if self.fields.contains(&name) {
                            self.temp = Temp::Node;
                            break name;
                        }
                        self.de.escape();
                    }
                    // @ 之前的部分是真正的名字，用这个名字搜索连续的一组
                    else {
                        let name_bytes = &name.as_bytes()[..pre_len];
                        let name = unsafe { core::str::from_utf8_unchecked(name_bytes) };
                        let (group, len, next) = c.take_group_on(self.de.dtb, name);
                        self.de.cursor = next;
                        if self.fields.contains(&name) {
                            self.temp = Temp::Group(group, len, name.len());
                            break name;
                        }
                    }
                }
                // 属性条目
                Cursor::Prop(c) => {
                    let (name, next) = c.name_on(self.de.dtb);
                    self.de.cursor = next;
                    if self.fields.contains(&name) {
                        self.temp = Temp::Prop(c);
                        break name;
                    }
                }
                // 截止符，结构体解析完成
                Cursor::End => {
                    self.de.cursor.step_n(1);
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
        match self.temp {
            Temp::Node => {
                // 键是独立节点名字，递归
                seed.deserialize(&mut *self.de)
            }
            Temp::Group(cursor, len_item, len_name) => {
                // 键是组名字，构造组反序列化器
                seed.deserialize(&mut GroupDeserializer {
                    dtb: self.de.dtb,
                    cursor,
                    len_item,
                    len_name,
                })
            }
            Temp::Prop(cursor) => {
                // 键是属性名字，构造属性反序列化器
                seed.deserialize(&mut BorrowedValueDeserializer {
                    dtb: self.de.dtb,
                    cursor,
                })
            }
        }
    }
}
