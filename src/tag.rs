use crate::common::{FDT_BEGIN_NODE, FDT_END, FDT_END_NODE, FDT_NOP, FDT_PROP};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Tags<'a> {
    pub(crate) structure: &'a [u8],
    pub(crate) string_table: &'a [u8],
    pub(crate) cur: usize,
    pub(crate) offset_from_file_begin: usize,
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
