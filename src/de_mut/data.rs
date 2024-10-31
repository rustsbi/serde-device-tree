use super::{BodyCursor, Cursor};
use super::{DtError, PropCursor, RefDtb, RegConfig};

use core::marker::PhantomData;
use serde::{de, Deserialize};

#[derive(Clone, Copy, Debug)]
pub(super) enum ValueCursor {
    Prop(BodyCursor, PropCursor),
    Body(BodyCursor),
}

#[derive(Clone, Copy)]
pub(super) struct ValueDeserializer<'de> {
    pub dtb: RefDtb<'de>,
    pub reg: RegConfig,
    pub cursor: ValueCursor,
}

impl<'de> Deserialize<'de> for ValueDeserializer<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de, 'b> {
            marker: PhantomData<ValueDeserializer<'b>>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de, 'b> de::Visitor<'de> for Visitor<'de, 'b> {
            type Value = ValueDeserializer<'b>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "struct ValueDeserializer")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                Ok(unsafe {
                    *(*(core::ptr::addr_of!(deserializer) as *const _ as *const &ValueDeserializer))
                })
            }
        }

        serde::Deserializer::deserialize_newtype_struct(
            deserializer,
            super::VALUE_DESERIALIZER_NAME,
            Visitor {
                marker: PhantomData,
                lifetime: PhantomData,
            },
        )
    }
}

impl<'de> de::Deserializer<'de> for &mut ValueDeserializer<'de> {
    type Error = DtError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unimplemented!("any")
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if let ValueCursor::Prop(_, cursor) = self.cursor {
            let val = cursor.map_on(self.dtb, |data| {
                if data.is_empty() {
                    true
                } else {
                    todo!("&[u8] -> bool")
                }
            });
            return visitor.visit_bool(val);
        }
        unreachable!("Node -> bool");
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("i8")
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("i16")
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("i32")
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("i64")
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("u8")
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("u16")
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if let ValueCursor::Prop(_, cursor) = self.cursor {
            return visitor.visit_u32(cursor.map_u32_on(self.dtb)?);
        }
        unreachable!("node -> u32");
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("u64")
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("f32")
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("f64")
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("char")
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("str");
    }

    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("string")
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if let ValueCursor::Prop(_, cursor) = self.cursor {
            let data = cursor.data_on(self.dtb);
            return visitor.visit_borrowed_bytes(data);
        }
        unreachable!("node -> bytes");
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("byte_buf")
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.cursor {
            ValueCursor::Prop(_, cursor) => {
                let data = cursor.data_on(self.dtb);
                if data.is_empty() {
                    visitor.visit_none()
                } else {
                    visitor.visit_some(self)
                }
            }
            ValueCursor::Body(_) => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if name == super::VALUE_DESERIALIZER_NAME {
            return visitor.visit_newtype_struct(self);
        }
        match self.cursor {
            ValueCursor::Prop(_, cursor) => match name {
                "StrSeq" => {
                    let inner = super::str_seq::Inner {
                        dtb: self.dtb,
                        cursor,
                    };
                    visitor.visit_borrowed_bytes(unsafe {
                        core::slice::from_raw_parts(
                            &inner as *const _ as *const u8,
                            core::mem::size_of_val(&inner),
                        )
                    })
                }
                "Reg" => {
                    let inner = super::reg::Inner {
                        dtb: self.dtb,
                        reg: self.reg,
                        cursor,
                    };
                    visitor.visit_borrowed_bytes(unsafe {
                        core::slice::from_raw_parts(
                            &inner as *const _ as *const u8,
                            core::mem::size_of_val(&inner),
                        )
                    })
                }
                _ => visitor.visit_newtype_struct(self),
            },
            ValueCursor::Body(_) => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        use super::{StructAccess, StructAccessType, Temp};
        match self.move_on() {
            Cursor::Title(c) => {
                let (name, _) = c.split_on(self.dtb);
                let cursor = match self.cursor {
                    ValueCursor::Body(cursor) => cursor,
                    _ => unreachable!(""),
                };

                let pre_len = name.as_bytes().iter().take_while(|b| **b != b'@').count();
                let name_bytes = &name.as_bytes()[..pre_len];
                let name = unsafe { core::str::from_utf8_unchecked(name_bytes) };

                visitor.visit_seq(StructAccess {
                    access_type: StructAccessType::Seq(name),
                    temp: Temp::Node(cursor, cursor),
                    de: self,
                })
            }
            _ => unreachable!("seq request on a none seq cursor"),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("tuple")
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("tuple_struct")
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        use super::{StructAccess, StructAccessType, Temp};
        if let ValueCursor::Body(cursor) = self.cursor {
            return visitor.visit_map(StructAccess {
                access_type: StructAccessType::Map(false),
                temp: Temp::Node(cursor, cursor),
                de: self,
            });
        };
        unreachable!("Prop -> map")
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
        use super::{StructAccess, StructAccessType, Temp};
        if let ValueCursor::Body(cursor) = self.cursor {
            return visitor.visit_map(StructAccess {
                access_type: StructAccessType::Struct(fields),
                temp: Temp::Node(cursor, cursor),
                de: self,
            });
        };
        unreachable!("Prop -> struct {_name} {fields:?}")
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
        unreachable!("enum")
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("identifier")
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        unreachable!("ignored_any")
    }
}

impl ValueDeserializer<'_> {
    #[inline]
    pub fn move_on(&mut self) -> super::Cursor {
        if let ValueCursor::Body(ref mut cursor) = self.cursor {
            return cursor.move_on(self.dtb);
        };
        unreachable!("move_on prop cursor");
    }
    #[inline]
    pub fn step_n(&mut self, n: usize) {
        if let ValueCursor::Body(ref mut cursor) = self.cursor {
            return cursor.step_n(n);
        };
        unreachable!("step_n prop cursor");
    }
    #[inline]
    pub fn is_complete_on(&self) -> bool {
        if let ValueCursor::Body(cursor) = self.cursor {
            return cursor.is_complete_on(self.dtb);
        };
        unreachable!("is_complete_on prop cursor");
    }
    #[inline]
    pub fn file_index_on(&self) -> usize {
        if let ValueCursor::Body(cursor) = self.cursor {
            return cursor.file_index_on(self.dtb);
        };
        unreachable!("file_index_on prop cursor");
    }
}
