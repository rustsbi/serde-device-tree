// Copyright (c) 2021 HUST IoT Security Lab
// serde_device_tree is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//          http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! Deserialize device tree data to a Rust data structure.

use crate::{
    common::*,
    error::{Error, Result},
};
use core::iter::Peekable;
use serde::de;

/// Deserialize an instance of type `T` from raw pointer of device tree blob.
///
/// This function is useful in developing device tree compatible firmware
/// or operating system kernels to parse structure from previous bootloading
/// stage.
///
/// # Safety
///
/// TODO
///
/// # Example
///
/// ```
/// # static DEVICE_TREE: &'static [u8] = include_bytes!("../examples/hifive-unmatched-a00.dtb");
/// # let dtb_pa = DEVICE_TREE.as_ptr() as usize;
/// use serde_derive::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// struct Tree<'a> {
///     #[serde(borrow)]
///     chosen: Option<Chosen<'a>>,
/// }
///
/// #[derive(Debug, Deserialize)]
/// #[serde(rename_all = "kebab-case")]
/// struct Chosen<'a> {
///     stdout_path: Option<&'a str>,
/// }
///
/// let tree: Tree = unsafe { serde_device_tree::from_raw(dtb_pa as *const u8) }
///     .expect("parse device tree");
/// if let Some(chosen) = tree.chosen {
///     if let Some(stdout_path) = chosen.stdout_path {
///         println!("stdout path: {}", stdout_path);
///     }
/// }
/// ```
pub unsafe fn from_raw<'de, T>(ptr: *const u8) -> Result<T>
where
    T: de::Deserialize<'de>,
{
    // read header
    let header = &*(ptr as *const Header);
    header.verify()?;

    let total_size = u32::from_be(header.total_size);
    let raw_data_len = (total_size - HEADER_LEN) as usize;
    let ans_ptr = core::ptr::from_raw_parts(ptr as *const (), raw_data_len);
    let device_tree: &DeviceTree = &*ans_ptr;
    let tags = device_tree.tags();
    let mut d = Deserializer {
        tags: tags.peekable(),
    };
    let ret = T::deserialize(&mut d)?;
    Ok(ret)
}

#[derive(Debug)]
struct DeviceTree {
    header: Header,
    data: [u8],
}

impl DeviceTree {
    pub fn tags(&self) -> Tags {
        let structure_addr = (u32::from_be(self.header.off_dt_struct) - HEADER_LEN) as usize;
        let structure_len = u32::from_be(self.header.size_dt_struct) as usize;
        let strings_addr = (u32::from_be(self.header.off_dt_strings) - HEADER_LEN) as usize;
        let strings_len = u32::from_be(self.header.size_dt_strings) as usize;
        Tags {
            structure: &self.data[structure_addr..structure_addr + structure_len],
            string_table: &self.data[strings_addr..strings_addr + strings_len],
            cur: 0,
            offset_from_file_begin: structure_addr,
        }
    }
}

#[derive(Debug, Clone)]
struct Tags<'a> {
    structure: &'a [u8],
    string_table: &'a [u8],
    cur: usize,
    offset_from_file_begin: usize,
}

#[inline]
fn align_up_u32(val: usize) -> usize {
    val + (4 - (val % 4)) % 4
}

impl<'a> Tags<'a> {
    #[inline]
    fn file_index(&self) -> usize {
        self.cur + self.offset_from_file_begin
    }
    #[inline]
    fn read_cur_u32(&mut self) -> Result<u32> {
        if self.cur >= (u32::MAX - 4) as usize {
            return Err(Error::u32_index_space_overflow(
                self.cur as u32,
                self.file_index(),
            ));
        }
        let ans = u32::from_be_bytes([
            self.structure[self.cur],
            self.structure[self.cur + 1],
            self.structure[self.cur + 2],
            self.structure[self.cur + 3],
        ]);
        self.cur += 4;
        Ok(ans)
    }
    #[inline]
    fn read_string0_align(&mut self) -> Result<&'a [u8]> {
        let begin = self.cur;
        while self.cur < self.structure.len() {
            if self.structure[self.cur] == b'\0' {
                let end = self.cur;
                self.cur = align_up_u32(end + 1);
                return Ok(&self.structure[begin..end]);
            }
            self.cur += 1;
        }
        Err(Error::string_eof_unpexpected(self.file_index()))
    }
    #[inline]
    fn read_slice_align(&mut self, len: u32) -> Result<&'a [u8]> {
        let begin = self.cur;
        let end = self.cur + len as usize;
        if end > self.structure.len() {
            let remaining_length = self.structure.len() as u32 - begin as u32;
            return Err(Error::slice_eof_unpexpected(
                len,
                remaining_length,
                self.file_index(),
            ));
        }
        self.cur = align_up_u32(end);
        Ok(&self.structure[begin..end])
    }
    #[inline]
    fn read_table_string(&mut self, pos: u32) -> Result<&'a [u8]> {
        let begin = pos as usize;
        if begin >= self.string_table.len() {
            let bound_offset = self.string_table.len() as u32;
            return Err(Error::table_string_offset(
                pos,
                bound_offset,
                self.file_index(),
            ));
        }
        let mut cur = begin;
        while cur < self.string_table.len() {
            if self.string_table[cur] == b'\0' {
                return Ok(&self.string_table[begin..cur]);
            }
            cur += 1;
        }
        Err(Error::table_string_offset(
            pos,
            cur as u32,
            self.file_index(),
        ))
    }
}

impl<'a> Iterator for Tags<'a> {
    type Item = Result<(Tag<'a>, usize)>; // Tag, byte index from file begin
    fn next(&mut self) -> Option<Self::Item> {
        if self.cur > self.structure.len() - core::mem::size_of::<u32>() {
            return Some(Err(Error::tag_eof_unexpected(
                self.cur as u32,
                self.structure.len() as u32,
                self.file_index(),
            )));
        }
        let ans = loop {
            match self.read_cur_u32() {
                // begin of structure tag
                Ok(FDT_BEGIN_NODE) => break Some(self.read_string0_align().map(Tag::Begin)),
                Ok(FDT_PROP) => {
                    let val_size = match self.read_cur_u32() {
                        Ok(v) => v,
                        Err(e) => break Some(Err(e)),
                    };
                    let name_offset = match self.read_cur_u32() {
                        Ok(v) => v,
                        Err(e) => break Some(Err(e)),
                    };
                    // get value slice
                    let val = match self.read_slice_align(val_size) {
                        Ok(slice) => slice,
                        Err(e) => break Some(Err(e)),
                    };

                    // lookup name in strings table
                    let prop_name = match self.read_table_string(name_offset) {
                        Ok(slice) => slice,
                        Err(e) => break Some(Err(e)),
                    };
                    break Some(Ok(Tag::Prop(val, prop_name)));
                }
                Ok(FDT_END_NODE) => break Some(Ok(Tag::End)),
                Ok(FDT_NOP) => self.cur += 4,
                Ok(FDT_END) => break None,
                Ok(invalid) => break Some(Err(Error::invalid_tag_id(invalid, self.file_index()))),
                Err(e) => break Some(Err(e)),
            }
        };
        match ans {
            Some(Ok(tag)) => Some(Ok((tag, self.file_index()))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Tag<'a> {
    Begin(&'a [u8]),
    Prop(&'a [u8], &'a [u8]),
    End,
}

#[derive(Debug, Clone)]
pub struct Deserializer<'a> {
    tags: Peekable<Tags<'a>>,
}

impl<'a> Deserializer<'a> {
    fn next_tag(&mut self) -> Result<Option<(Tag<'a>, usize)>> {
        self.tags.next().transpose()
    }
    fn peek_tag(&mut self) -> Result<Option<Tag<'a>>> {
        match self.tags.peek() {
            Some(Ok((t, _i))) => Ok(Some(*t)),
            Some(Err(e)) => Err(e.clone()),
            None => Ok(None),
        }
    }
    fn peek_tag_index(&mut self) -> Result<Option<&(Tag<'a>, usize)>> {
        match self.tags.peek() {
            Some(Ok(t)) => Ok(Some(t)),
            Some(Err(e)) => Err(e.clone()),
            None => Ok(None),
        }
    }
    fn eat_tag(&mut self) -> Result<()> {
        match self.tags.next() {
            Some(Ok(_t)) => Ok(()),
            Some(Err(e)) => Err(e),
            None => Ok(()),
        }
    }
}

impl<'de, 'b> de::Deserializer<'de> for &'b mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.peek_tag()? {
            Some(Tag::Prop(_, value_slice)) => {
                if value_slice.is_empty() {
                    self.deserialize_bool(visitor)
                } else if value_slice.len() == 4 {
                    self.deserialize_u32(visitor)
                } else {
                    self.deserialize_bytes(visitor) // by default, it's bytes
                }
            }
            Some(Tag::Begin(_name_slice)) => self.deserialize_map(visitor),
            Some(Tag::End) => todo!(),
            _ => todo!(),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.peek_tag_index()? {
            Some((Tag::Prop(value_slice, _name_slice), _file_index)) => {
                if value_slice.is_empty() {
                    self.eat_tag()?;
                    visitor.visit_bool(true)
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.peek_tag_index()? {
            Some((Tag::Prop(value_slice, _name_slice), file_index)) => {
                let value = match value_slice {
                    [a, b, c, d] => u32::from_be_bytes([*a, *b, *c, *d]),
                    _ => return Err(Error::invalid_serde_type_length(4, *file_index)),
                };
                self.eat_tag()?;
                visitor.visit_u32(value)
            }
            _ => todo!(),
        }
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.peek_tag_index()? {
            Some((Tag::Prop(value_slice, _name_slice), file_index)) => {
                let s =
                    core::str::from_utf8(value_slice).map_err(|e| Error::utf8(e, *file_index))?;
                let value = visitor.visit_borrowed_str(s)?;
                self.eat_tag()?;
                Ok(value)
            }
            _ => todo!(),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        match self.peek_tag()? {
            Some(Tag::Prop(value_slice, _name_slice)) => {
                let value = visitor.visit_borrowed_bytes(value_slice)?;
                self.eat_tag()?;
                Ok(value)
            }
            _ => todo!(),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = (name, visitor);
        todo!()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = (name, visitor);
        todo!()
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = visitor;
        todo!()
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = (len, visitor);
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = (name, len, visitor);
        todo!()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if let Some((Tag::Begin(_name_slice), _file_index)) = self.next_tag()? {
            let ret = visitor.visit_map(MapVisitor::new(self))?;
            if let Some((Tag::End, _file_index)) = self.next_tag()? {
                Ok(ret)
            } else {
                Err(Error::expected_struct_end())
            }
        } else {
            Err(Error::expected_struct_begin())
        }
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = (name, fields);
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let _ = (name, variants, visitor);
        todo!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if let Some((Tag::Begin(name_slice), file_index)) = self.peek_tag_index()? {
            let s = core::str::from_utf8(name_slice).map_err(|e| Error::utf8(e, *file_index))?;
            visitor.visit_str(s)
        } else {
            todo!()
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        if let Some(tag) = self.peek_tag()? {
            match tag {
                Tag::Begin(_) => {
                    self.eat_tag()?;
                    let mut depth = 0;
                    while let Some((tag, _file_index)) = self.next_tag()? {
                        match tag {
                            Tag::Begin(_) => depth += 1,
                            Tag::End => {
                                if depth == 0 {
                                    break;
                                } else {
                                    depth -= 1
                                }
                            }
                            Tag::Prop(_, _) => {}
                        }
                    }
                }
                Tag::End => todo!(),
                Tag::Prop(_, _) => self.eat_tag()?,
            }
        }
        visitor.visit_unit()
    }
}

struct MapVisitor<'de, 'b> {
    de: &'b mut Deserializer<'de>,
}

impl<'de, 'b> MapVisitor<'de, 'b> {
    fn new(de: &'b mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'de, 'b> de::MapAccess<'de> for MapVisitor<'de, 'b> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        match self.de.peek_tag()? {
            Some(Tag::Prop(_value_slice, name_slice)) => seed
                .deserialize(serde::de::value::BorrowedBytesDeserializer::new(name_slice))
                .map(Some),
            Some(Tag::Begin(name_slice)) => seed
                .deserialize(serde::de::value::BorrowedBytesDeserializer::new(name_slice))
                .map(Some),
            Some(Tag::End) => Ok(None),
            None => Err(Error::no_remaining_tags()),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.de.peek_tag()? {
            Some(Tag::Prop(_value_slice, _name_slice)) => seed.deserialize(&mut *self.de),
            Some(Tag::Begin(_name_slice)) => seed.deserialize(&mut *self.de),
            Some(Tag::End) => panic!(),
            None => Err(Error::no_remaining_tags()),
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    use alloc::format;
    #[cfg(any(feature = "std", feature = "alloc"))]
    use serde_derive::Deserialize;
    #[cfg(feature = "std")]
    use std::format;

    #[cfg(any(feature = "std", feature = "alloc"))]
    #[test]
    fn error_invalid_magic() {
        static DEVICE_TREE: &'static [u8] = &[0x11, 0x22, 0x33, 0x44]; // not device tree blob format
        let ptr = DEVICE_TREE.as_ptr();

        #[derive(Debug, Deserialize)]
        struct Tree {}

        let ans: Result<Tree, _> = unsafe { super::from_raw(ptr) };
        let err = ans.unwrap_err();
        assert_eq!(
            "Error(invalid magic, value: 287454020, index: 0)",
            format!("{}", err)
        );
    }
}
