use super::{PropCursor, RefDtb};
use core::{fmt::Debug, marker::PhantomData, mem::MaybeUninit};
use serde::{de, Deserialize};

pub struct StrSeq<'de> {
    dtb: RefDtb<'de>,
    cursor: PropCursor,
}

pub struct StrSeqIter<'de, 'b> {
    seq: &'b StrSeq<'de>,
    i: usize,
}

impl<'de, 'b> Deserialize<'de> for StrSeq<'b> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de, 'b> {
            marker: PhantomData<StrSeq<'b>>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de, 'b> de::Visitor<'de> for Visitor<'de, 'b> {
            type Value = StrSeq<'b>;

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
            "StrSeq",
            Visitor {
                marker: PhantomData,
                lifetime: PhantomData,
            },
        )
    }
}

impl<'de> StrSeq<'de> {
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
        res.cursor.operate_on(res.dtb, |data| {
            let mut i = data.len() - 1;
            for j in (0..data.len() - 1).rev() {
                if data[j] == b'\0' {
                    data[i] = (i - j - 1) as _;
                    i = j;
                }
            }
            data[i] = i as u8;
        });
        res
    }

    pub fn iter<'b>(&'b self) -> StrSeqIter<'de, 'b> {
        StrSeqIter {
            seq: self,
            i: self.cursor.map_on(self.dtb, |data| data.len()),
        }
    }
}

impl Debug for StrSeq<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iter = self.iter();
        if let Some(s) = iter.next() {
            write!(f, "[\"{s}\"",)?;
            for s in iter {
                write!(f, ", \"{s}\"")?;
            }
            write!(f, "]")
        } else {
            write!(f, "[]")
        }
    }
}

impl<'de, 'b> Iterator for StrSeqIter<'de, 'b> {
    type Item = &'b str;

    fn next(&mut self) -> Option<Self::Item> {
        self.seq.cursor.map_on(self.seq.dtb, |data| {
            if self.i == 0 {
                None
            } else {
                let idx = self.i - 1;
                let len = data[idx] as usize;
                self.i = idx - len;
                let ptr = data[self.i..].as_ptr();
                unsafe {
                    let s = core::slice::from_raw_parts(ptr, len);
                    Some(core::str::from_utf8_unchecked(s))
                }
            }
        })
    }
}

impl Drop for StrSeq<'_> {
    fn drop(&mut self) {
        self.cursor.operate_on(self.dtb, |data| {
            let mut idx = data.len() - 1;
            loop {
                let len = data[idx] as usize;
                data[idx] = 0;
                if idx > len {
                    idx -= len + 1;
                } else {
                    break;
                }
            }
        })
    }
}
