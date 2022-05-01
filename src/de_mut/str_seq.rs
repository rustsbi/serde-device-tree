use super::{PropCursor, RefDtb};
use core::{fmt::Debug, marker::PhantomData, mem::MaybeUninit};
use serde::{de, Deserialize};

/// 一组 '\0' 分隔字符串的映射。
///
/// `compatible = "sifive,clint0","riscv,clint0";`
/// 这样的一条属性会被编译为两个连续的 '\0' 结尾字符串。
/// `StrSeq` 可以自动将它们分开。
///
/// `iter` 方法会创建一个迭代器，用于依次访问这些字符串。
/// 根据实现，迭代器会以从右到左的顺序返回这些字符串。
///
/// 构建时，所有字符串被遍历，所有分隔位置被记录下来。
/// 这需要修改 DTB 上字符串所在位置的内存，因此需要这块内存的写权限。
/// 如果要以其他方式解析 DTB，先将 `StrSeq` 释放，否则可能引发错误。
pub struct StrSeq<'de>(Inner<'de>);

/// '\0' 分隔字符串组迭代器。
pub struct StrSeqIter<'de> {
    data: &'de [u8],
    i: usize,
}

pub(super) struct Inner<'de> {
    pub dtb: RefDtb<'de>,
    pub cursor: PropCursor,
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
                    Err(E::invalid_length(
                        v.len(),
                        &"`StrSeq` is copied with wrong size.",
                    ))
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
        res.0.cursor.operate_on(res.0.dtb, |data| {
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

    /// 构造一个可访问每个字符串的迭代器。
    pub fn iter<'b>(&'b self) -> StrSeqIter<'de> {
        let data = self.0.cursor.data_on(self.0.dtb);
        StrSeqIter {
            data,
            i: data.len(),
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

impl<'de> Iterator for StrSeqIter<'de> {
    type Item = &'de str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i == 0 {
            None
        } else {
            let idx = self.i - 1;
            let len = self.data[idx] as usize;
            self.i = idx - len;
            let ptr = self.data[self.i..].as_ptr();
            unsafe {
                let s = core::slice::from_raw_parts(ptr, len);
                Some(core::str::from_utf8_unchecked(s))
            }
        }
    }
}

impl Drop for StrSeq<'_> {
    fn drop(&mut self) {
        self.0.cursor.operate_on(self.0.dtb, |data| {
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
