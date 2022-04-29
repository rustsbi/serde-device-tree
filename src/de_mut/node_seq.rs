use super::{BodyCursor, GroupCursor, RefDtb, StructDeserializer};
use core::{fmt::Debug, marker::PhantomData, mem::MaybeUninit};
use serde::{de, Deserialize};

pub struct NodeSeq<'de, T> {
    dtb: RefDtb<'de>,
    cursor: GroupCursor,
    len_item: usize,
    len_name: usize,
    _phantom: PhantomData<T>,
}

pub struct NodeSeqIter<'de, 'b, T> {
    seq: &'b NodeSeq<'de, T>,
    cursor: GroupCursor,
    i: usize,
}

pub struct NodeSeqItem<'de, T> {
    dtb: RefDtb<'de>,
    body: BodyCursor,
    at: &'de str,
    _phantom: PhantomData<T>,
}

impl<'de, 'b, T> Deserialize<'de> for NodeSeq<'b, T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de, 'b, T> {
            marker: PhantomData<NodeSeq<'b, T>>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de, 'b, T> de::Visitor<'de> for Visitor<'de, 'b, T> {
            type Value = NodeSeq<'b, T>;

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
                    todo!("{} != {}", v.len(), core::mem::size_of::<Self::Value>());
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

impl<'de, T> NodeSeq<'de, T> {
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

    pub fn len(&self) -> usize {
        self.len_item
    }

    pub fn is_empty(&self) -> bool {
        self.len_item == 0
    }

    pub fn iter<'b>(&'b self) -> NodeSeqIter<'de, 'b, T> {
        NodeSeqIter {
            seq: self,
            cursor: self.cursor,
            i: 0,
        }
    }
}

impl<T: Debug> Debug for NodeSeq<'_, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "todo")
    }
}

impl<T> Drop for NodeSeq<'_, T> {
    fn drop(&mut self) {
        self.cursor.drop_on(self.dtb, self.len_item);
    }
}

impl<'de, 'b, T> Iterator for NodeSeqIter<'de, 'b, T> {
    type Item = NodeSeqItem<'de, T>;

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
                body,
                at: unsafe { core::str::from_utf8_unchecked(&name[self.seq.len_name + 1..]) },
                _phantom: PhantomData,
            })
        }
    }
}

impl<'de, T: Deserialize<'de>> NodeSeqItem<'de, T> {
    pub fn at(&self) -> &str {
        self.at
    }

    pub fn deserialize(&self) -> T {
        T::deserialize(&mut StructDeserializer {
            dtb: self.dtb,
            cursor: self.body,
        })
        .unwrap()
    }
}
