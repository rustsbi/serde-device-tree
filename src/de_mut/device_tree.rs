use super::*;
use crate::{common::Header, Error};

/// 设备树。
#[derive(Debug)]
pub(super) struct DeviceTree {
    /// 光标，指示当前解析位置。
    pub cursor: usize,
    /// 设备树的结构块。
    pub structure: &'static mut [StructureBlock],
    /// 设备树的字符串块。
    pub strings: &'static [u8],
}

impl DeviceTree {
    /// 从指针构造设备树。
    pub unsafe fn from_raw_ptr(ptr: *mut u8) -> Result<Self, Error> {
        let header = &*(ptr as *const Header);
        header.verify()?;

        let off_dt_struct = u32::from_be(header.off_dt_struct);
        let size_dt_struct = u32::from_be(header.size_dt_struct);
        let off_dt_strings = u32::from_be(header.off_dt_strings);
        let size_dt_strings = u32::from_be(header.size_dt_strings);

        let ptr_dt_struct = ptr.add(off_dt_struct as _) as *mut StructureBlock;
        let size_dt_struct = size_dt_struct as usize / U32_LEN;
        let ptr_dt_strings = ptr.add(off_dt_strings as _);
        let size_dt_strings = size_dt_strings as usize;

        Ok(Self {
            cursor: 0,
            structure: core::slice::from_raw_parts_mut(ptr_dt_struct.add(2), size_dt_struct - 3),
            strings: core::slice::from_raw_parts(ptr_dt_strings, size_dt_strings),
        })
    }

    /// 从内存块构造设备树。
    ///
    /// 可以使用结构块的一部分构造子设备树。
    pub fn from_parts(structure: &[u8], strings: &[u8]) -> Self {
        let ptr_dt_struct = structure.as_ptr() as *mut _;
        let size_dt_struct = structure.len() / U32_LEN;

        let ptr_dt_strings = strings.as_ptr();
        let size_dt_strings = strings.len();

        Self {
            cursor: 0,
            structure: unsafe { core::slice::from_raw_parts_mut(ptr_dt_struct, size_dt_struct) },
            strings: unsafe { core::slice::from_raw_parts(ptr_dt_strings, size_dt_strings) },
        }
    }
}

impl DeviceTree {
    /// 解析作为节点名的 '\0' 结尾字符串。
    fn next_cstr(&mut self) -> Result<&'static str, Error> {
        let begin = self.cursor;
        self.structure[begin..]
            .iter()
            .enumerate()
            .find(|(_, block)| block.0[3] == OF_DT_END_STR)
            .map(|(i, block)| {
                self.cursor += i + 1;
                unsafe {
                    core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                        self.structure[begin..].as_ptr() as *const _,
                        i * U32_LEN
                            + match block.0 {
                                [0, _, _, _] => 0,
                                [_, 0, _, _] => 1,
                                [_, _, 0, _] => 2,
                                [_, _, _, _] => 3,
                            },
                    ))
                }
            })
            .ok_or_else(|| Error::string_eof_unpexpected(begin * U32_LEN))
    }

    /// 解析属性条目。
    fn next_prop(&mut self) -> Result<(&'static str, &'static [u8]), Error> {
        match self.structure[self.cursor..] {
            [data_len, name_off, ..] => {
                let data_len = u32::from_be_bytes(data_len.0) as usize;
                let data = unsafe {
                    core::slice::from_raw_parts(
                        self.structure[self.cursor + 2..].as_ptr() as _,
                        data_len,
                    )
                };

                let name_off = u32::from_be_bytes(name_off.0) as usize;
                let name = self.strings[name_off..]
                    .iter()
                    .enumerate()
                    .find(|(_, b)| **b == 0)
                    .map(|(i, _)| unsafe {
                        core::str::from_utf8_unchecked(&self.strings[name_off..][..i])
                    })
                    .ok_or_else(|| todo!())?;

                self.cursor += 2 + (data_len + U32_LEN - 1) / U32_LEN;
                Ok((name, data))
            }
            _ => todo!(),
        }
    }

    /// 解析一组同级同类节点。
    fn next_multiple(&mut self, name: &str, begin: usize) -> Result<&'static mut [u8], Error> {
        let name_bytes = name.as_bytes();
        let name_blocks = (name_bytes.len() + U32_LEN - 1) / U32_LEN;
        self.skip_node()?;
        while let [StructureBlock(block), ..] = self.structure[self.cursor..] {
            match block {
                OF_DT_BEGIN_NODE => {
                    let name_bytes_ = unsafe {
                        core::slice::from_raw_parts(
                            self.structure[self.cursor + 1..].as_ptr() as *const u8,
                            name_bytes.len() + 1,
                        )
                    };
                    match name_bytes_ {
                        [name @ .., at] if name == name_bytes && *at == b'@' => {
                            self.cursor += 1 + name_blocks;
                            self.skip_cstr()?;
                            self.skip_node()?;
                        }
                        _ => break,
                    }
                }
                OF_DT_END_NODE | OF_DT_PROP => break,
                OF_DT_NOP => self.cursor += 1,
                _ => todo!(),
            }
        }
        Ok(unsafe {
            core::slice::from_raw_parts_mut(
                self.structure[begin..].as_ptr() as *mut _,
                (self.cursor - begin) * U32_LEN,
            )
        })
    }

    /// 设备树全部解析完成。
    pub fn end(&self) -> bool {
        self.cursor >= self.structure.len()
    }

    /// 解析出下一个反序列化单元。
    pub fn next(&mut self) -> Result<Tag, Error> {
        let begin = self.cursor;
        self.structure[begin..]
            .iter()
            .map(|block| block.0)
            .enumerate()
            .find(|(_, b)| *b != OF_DT_NOP)
            .ok_or_else(|| {
                Error::tag_eof_unexpected(
                    begin as u32,
                    self.structure.len() as u32,
                    begin * U32_LEN,
                )
            })
            .and_then(|(i, tag)| {
                let mark = self.cursor + i;
                self.cursor = mark + 1;
                match tag {
                    OF_DT_BEGIN_NODE => {
                        let name = self.next_cstr()?;
                        if let Some((name, _)) = name.split_once('@') {
                            let block = self.next_multiple(name, mark)?;
                            Ok(Tag::MultipleBlock(name, block))
                        } else {
                            Ok(Tag::Begin(name))
                        }
                    }
                    OF_DT_PROP => {
                        let (key, value) = self.next_prop()?;
                        Ok(Tag::Prop(key, value))
                    }
                    OF_DT_END_NODE => Ok(Tag::End),
                    _ => Err(Error::invalid_tag_id(
                        u32::from_be_bytes(tag),
                        begin * U32_LEN,
                    )),
                }
            })
    }

    /// 从 BEGIN 标签后跳过一个 '\0' 结尾字符串。
    pub fn skip_cstr(&mut self) -> Result<(), Error> {
        while let Some(StructureBlock(block)) = self.structure.get(self.cursor) {
            self.cursor += 1;
            if block[3] == OF_DT_END_STR {
                return Ok(());
            }
        }
        Ok(())
    }

    /// 从节点名后跳过节点内部结构。
    pub fn skip_node(&mut self) -> Result<(), Error> {
        let mut level = 1;
        'outer: while let Some(StructureBlock(block)) = self.structure.get(self.cursor) {
            match *block {
                OF_DT_BEGIN_NODE => {
                    self.cursor += 1;
                    level += 1;
                    while let Some(block) = self.structure.get(self.cursor) {
                        self.cursor += 1;
                        if let [_, _, _, OF_DT_END_STR] = block.0 {
                            continue 'outer;
                        }
                    }
                    todo!()
                }
                OF_DT_END_NODE => {
                    self.cursor += 1;
                    if level == 1 {
                        return Ok(());
                    }
                    level -= 1;
                }
                OF_DT_PROP => {
                    if let [_, data_len, _, ..] = self.structure[self.cursor..] {
                        let data_len = u32::from_be_bytes(data_len.0) as usize;
                        self.cursor += 3 + (data_len + U32_LEN - 1) / U32_LEN;
                    } else {
                        todo!()
                    }
                }
                OF_DT_NOP => {
                    self.cursor += 1;
                }
                [_, _, _, _] => todo!(),
            }
        }
        Ok(())
    }
}
