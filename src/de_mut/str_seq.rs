//! 无堆内存分配，且 O(1) 迭代的 '\0' 分隔字符串序列。

use core::{fmt::Debug, marker::PhantomData};
use serde::{de::Unexpected, Deserialize};

/// 字符串序列。
pub struct StrSeq {
    ptr: *mut u8,
    len: usize,
}

/// 字符串迭代器。
pub struct StrSeqIter<'a> {
    ptr: *const u8,
    len: usize,
    _lifetime: PhantomData<&'a ()>,
}

impl<'de> serde::Deserialize<'de> for StrSeq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let val: &'de str = Deserialize::deserialize(deserializer)?;
        unsafe {
            let ptr = val.as_ptr() as *mut u8;
            let len = val.as_bytes().len();
            let slice = core::slice::from_raw_parts_mut(ptr, len);

            let mut i = len - 1;
            if slice[i] != b'\0' {
                return Err(serde::de::Error::invalid_value(
                    Unexpected::Unsigned(slice[i] as _),
                    &"str must end with '\\0'",
                ));
            }

            let ptr = ptr.add(i);
            let mut len = 1;

            let mut j = i - 1;
            while j > 0 {
                if slice[j] == b'\0' {
                    slice[i] = (i - j - 1) as _;
                    i = j;
                    len += 1;
                }
                j -= 1;
            }
            slice[i] = i as _;

            Ok(Self { ptr, len })
        }
    }
}

impl Debug for StrSeq {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut cloned = self.iter();
        write!(f, "[\"{}\"", cloned.next().unwrap())?;
        for item in cloned {
            write!(f, ", \"{}\"", item)?;
        }
        write!(f, "]")
    }
}

impl Drop for StrSeq {
    fn drop(&mut self) {
        while self.len > 0 {
            unsafe {
                let len = *self.ptr as usize;
                *self.ptr = b'\0';
                self.ptr = self.ptr.sub(len + 1);
            };
            self.len -= 1;
        }
    }
}

impl StrSeq {
    pub fn iter(&self) -> StrSeqIter<'_> {
        StrSeqIter {
            ptr: self.ptr,
            len: self.len,
            _lifetime: PhantomData,
        }
    }
}

impl<'a> Iterator for StrSeqIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.len == 0 {
            None
        } else {
            let str = unsafe {
                let len = *self.ptr as usize;
                self.ptr = self.ptr.sub(len + 1);
                core::str::from_utf8_unchecked(core::slice::from_raw_parts(self.ptr.add(1), len))
            };
            self.len -= 1;
            Some(str)
        }
    }
}
