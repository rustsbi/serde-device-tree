use super::{BodyCursor, DtError, GroupCursor, RefDtb, RegConfig, StructDeserializer};
use core::{fmt::Debug, marker::PhantomData, mem::MaybeUninit};
use serde::{de, Deserialize};

/// 一组名字以 `@...` 区分，同类、同级且连续的节点的映射。
///
/// 在解析前，无法得知这种节点的数量，因此也无法为它们分配足够的空间，
/// 因此这些节点将延迟解析。
/// 迭代 `NodeSeq` 可获得一系列 [`NodeSeqItem`]，再调用 `deserialize` 方法分别解析每个节点。
pub struct NodeSeq<'de> {
    pub(super) dtb: RefDtb<'de>,
    pub(super) reg: RegConfig,
    pub(super) cursor: GroupCursor,
    pub(super) len_item: usize,
    pub(super) len_name: usize,
}

/// 连续节点迭代器。
pub struct NodeSeqIter<'de, 'b> {
    seq: &'b NodeSeq<'de>,
    cursor: GroupCursor,
    i: usize,
}

/// 连续节点对象。
pub struct NodeSeqItem<'de> {
    dtb: RefDtb<'de>,
    reg: RegConfig,
    body: BodyCursor,
    at: &'de str,
}

impl<'de, 'b> Deserialize<'de> for NodeSeq<'b> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de, 'b> {
            marker: PhantomData<NodeSeq<'b>>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de, 'b> de::Visitor<'de> for Visitor<'de, 'b> {
            type Value = NodeSeq<'b>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "struct StrSeq")
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
                        &"`NodeSeq` is copied with wrong size.",
                    ))
                }
            }
        }

        serde::Deserializer::deserialize_newtype_struct(
            deserializer,
            "NodeSeq",
            Visitor {
                marker: PhantomData,
                lifetime: PhantomData,
            },
        )
    }
}

impl<'de> NodeSeq<'de> {
    fn from_raw_parts(ptr: *const u8) -> Self {
        // 直接从指针拷贝
        let res = unsafe {
            let mut res = MaybeUninit::<Self>::uninit();
            core::ptr::copy_nonoverlapping(
                ptr,
                res.as_mut_ptr() as *mut _,
                core::mem::size_of::<Self>(),
            );
            res.assume_init()
        };
        // 初始化
        res.cursor.init_on(res.dtb, res.len_item, res.len_name);
        res
    }

    /// 连续节点总数。
    pub const fn len(&self) -> usize {
        self.len_item
    }

    /// 如果连续节点数量为零，返回 true。但连续节点数量不可能为零。
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// 获得节点迭代器。
    pub const fn iter<'b>(&'b self) -> NodeSeqIter<'de, 'b> {
        NodeSeqIter {
            seq: self,
            cursor: self.cursor,
            i: 0,
        }
    }
}

impl Debug for NodeSeq<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iter = self.iter();
        if let Some(s) = iter.next() {
            write!(f, "[@{}", s.at)?;
            for s in iter {
                write!(f, ", @{}", s.at)?;
            }
            write!(f, "]")
        } else {
            unreachable!("NodeSeq contains at least one node.")
        }
    }
}

impl Drop for NodeSeq<'_> {
    fn drop(&mut self) {
        self.cursor.drop_on(self.dtb, self.len_item);
    }
}

impl<'de, 'b> Iterator for NodeSeqIter<'de, 'b> {
    type Item = NodeSeqItem<'de>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.seq.len_item {
            None
        } else {
            self.i += 1;
            let (name, body) = self.cursor.name_on(self.seq.dtb);
            let off_next = self.cursor.offset_on(self.seq.dtb);
            self.cursor.step_n(off_next);
            Some(Self::Item {
                dtb: self.seq.dtb,
                reg: self.seq.reg,
                body,
                at: unsafe { core::str::from_utf8_unchecked(&name[self.seq.len_name + 1..]) },
            })
        }
    }
}

impl NodeSeqItem<'_> {
    /// 获得区分节点的序号。
    pub fn at(&self) -> &str {
        self.at
    }
}

impl<'de> NodeSeqItem<'de> {
    /// 反序列化一个节点的内容。
    pub fn deserialize<T: Deserialize<'de>>(&self) -> T {
        T::deserialize(&mut StructDeserializer {
            dtb: self.dtb,
            reg: self.reg,
            cursor: self.body,
        })
        .unwrap()
    }
}

impl<'de, 'b> de::Deserializer<'de> for &'b mut NodeSeq<'de> {
    type Error = DtError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("any")
    }

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("bool")
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

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("u32")
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

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("str")
    }

    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("string")
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("bytes")
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
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if name == "NodeSeq" {
            visitor.visit_borrowed_bytes(unsafe {
                core::slice::from_raw_parts(
                    self as *const _ as *const u8,
                    core::mem::size_of::<Self>(),
                )
            })
        } else {
            visitor.visit_newtype_struct(self)
        }
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
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("tuple_struct")
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
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("struct")
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
        todo!("identifer")
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        todo!("ignored_any")
    }
}
