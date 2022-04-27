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
use r#struct::StructDeserializer;
use structs::{RefDtb, StructureBlock};

use self::group::GroupDeserializer;

/// 只在栈上计算，实现设备树解析。
pub fn from_raw_mut<'de, T>(dtb: RefDtb<'de>) -> Result<T, DtError>
where
    T: de::Deserialize<'de>,
{
    let mut d = StructDeserializer {
        dtb,
        cursor: BodyCursor::ROOT,
    };
    T::deserialize(&mut d).and_then(|t| {
        if d.cursor.is_complete_on(dtb) {
            Ok(t)
        } else {
            todo!("end at {:?}", d.cursor)
        }
    })
}

struct StructAccess<'de, 'b> {
    fields: &'static [&'static str],
    temp: Temp,
    de: &'b mut StructDeserializer<'de>,
}

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
                Cursor::Title(c) => {
                    let (name, sub) = c.split_on(self.de.dtb);

                    let pre_len = name.as_bytes().iter().take_while(|b| **b != b'@').count();
                    if pre_len == name.as_bytes().len() {
                        self.de.cursor = sub;
                        if self.fields.contains(&name) {
                            self.temp = Temp::Node;
                            break name;
                        }
                        self.de.escape();
                    } else {
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
                Cursor::Prop(c) => {
                    let (name, next) = c.name_on(self.de.dtb);
                    self.de.cursor = next;
                    if self.fields.contains(&name) {
                        self.temp = Temp::Prop(c);
                        break name;
                    }
                }
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
                //
                seed.deserialize(&mut *self.de)
            }
            Temp::Group(cursor, len_item, len_name) => {
                //
                seed.deserialize(&mut GroupDeserializer {
                    dtb: self.de.dtb,
                    cursor,
                    len_item,
                    len_name,
                })
            }
            Temp::Prop(cursor) => {
                //
                seed.deserialize(&mut BorrowedValueDeserializer {
                    dtb: self.de.dtb,
                    cursor,
                })
            }
        }
    }
}
