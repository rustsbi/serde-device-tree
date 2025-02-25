use super::{PropCursor, RefDtb, ValueCursor};
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

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

pub(super) struct Inner<'de> {
    pub dtb: RefDtb<'de>,
    pub cursor: PropCursor,
}

/// '\0' 分隔字符串组迭代器。
pub struct StrSeqIter<'de> {
    data: &'de [u8],
}

impl<'de> Deserialize<'de> for StrSeq<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value_deserialzer = super::ValueDeserializer::deserialize(deserializer)?;

        let inner = Inner {
            dtb: value_deserialzer.dtb,
            cursor: match value_deserialzer.cursor {
                ValueCursor::Prop(_, cursor) => cursor,
                _ => {
                    unreachable!("StrSeq Deserialize should only be called by prop cursor")
                }
            },
        };

        Ok(Self(inner))
    }
}

impl<'de> StrSeq<'de> {
    /// 构造一个可访问每个字符串的迭代器。
    pub fn iter<'b>(&'b self) -> StrSeqIter<'de> {
        StrSeqIter {
            data: self.0.cursor.data_on(self.0.dtb),
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
        if self.data.is_empty() {
            None
        } else {
            let pos = self
                .data
                .iter()
                .position(|&x| x == b'\0')
                .unwrap_or(self.data.len());
            let (a, b) = self.data.split_at(pos + 1);
            self.data = b;
            // Remove \0 at end
            Some(unsafe { core::str::from_utf8_unchecked(&a[..a.len() - 1]) })
        }
    }
}

impl<'se> Serialize for StrSeq<'se> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Pass bytes directly for StrSeq.
        serializer.serialize_bytes(self.0.cursor.data_on(self.0.dtb))
    }
}
