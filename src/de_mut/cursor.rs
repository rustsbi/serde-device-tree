use super::{BLOCK_LEN, DtError, RefDtb, StructureBlock};
use core::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(super) struct AnyCursor<T: Type = Body>(usize, PhantomData<T>);

pub(super) type BodyCursor = AnyCursor<Body>;
pub(super) type TitleCursor = AnyCursor<Title>;
pub(super) type PropCursor = AnyCursor<Prop>;

pub(super) trait Type {}

#[derive(Clone, Copy, Debug)]
pub(super) struct Body {}
#[derive(Clone, Copy, Debug)]
pub(super) struct Title {}
#[derive(Clone, Copy, Debug)]
pub(super) struct Prop {}

impl Type for Body {}
impl Type for Title {}
impl Type for Prop {}

pub enum MoveResult {
    In,
    Out,
    Others,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MultiNodeCursor {
    pub start_cursor: BodyCursor,
    pub skip_cursor: BodyCursor,
    pub data_cursor: BodyCursor,
    #[allow(unused)]
    pub node_count: u32,
}

impl<T: Type> AnyCursor<T> {
    /// 移动 `n` 格。
    pub fn step_n(&mut self, len: usize) {
        self.0 += len;
    }

    /// 光标相对文件头的偏移。
    pub fn file_index_on(&self, dtb: RefDtb) -> usize {
        self.0 * BLOCK_LEN + dtb.borrow().off_dt_struct()
    }
}

impl BodyCursor {
    pub const ROOT: Self = Self(2, PhantomData);
    pub const STARTER: Self = Self(0, PhantomData);

    /// 移动到下一个项目。
    pub fn move_on(&mut self, dtb: RefDtb) -> Cursor {
        use StructureBlock as B;
        let structure = &dtb.borrow().structure;
        loop {
            match structure[self.0] {
                B::NODE_BEGIN => break Cursor::title(self.0),
                B::NODE_END => break Cursor::end(),
                B::PROP => break Cursor::prop(self.0),
                B::NOP => self.0 += 1,
                _ => todo!(),
            }
        }
    }

    /// 如果设备树已完全解析，返回 `true`。
    pub fn is_complete_on(&self, dtb: RefDtb) -> bool {
        self.0 + 1 == dtb.borrow().structure.len()
    }

    /// 跳过当前所在的字符串。
    pub fn skip_str_on(&mut self, dtb: RefDtb) {
        while let Some(block) = &dtb.borrow().structure.get(self.0) {
            self.0 += 1;
            if block.is_end_of_str() {
                return;
            }
        }
        todo!()
    }

    /// 移动指针至下一块
    pub fn move_next(&mut self, dtb: RefDtb) -> MoveResult {
        use StructureBlock as B;
        let structure = &dtb.borrow().structure;
        match structure[self.0] {
            // 下陷一级
            B::NODE_BEGIN => {
                self.0 += 1;
                self.skip_str_on(dtb);
                MoveResult::In
            }
            // 上浮一级
            B::NODE_END => {
                self.0 += 1;
                MoveResult::Out
            }
            // 属性项
            B::PROP => {
                if let [_, len_data, _, ..] = &structure[self.0..] {
                    self.0 += 3 + align(len_data.as_usize(), BLOCK_LEN);
                } else {
                    todo!()
                }
                MoveResult::Others
            }
            // 空白项
            B::NOP => {
                self.0 += 1;
                MoveResult::Others
            }
            _ => todo!("unknown block {}", structure[self.0]),
        }
    }

    /// 离开当前子树。
    pub fn escape_from(&mut self, dtb: RefDtb) {
        let mut level = 1;
        loop {
            match self.move_next(dtb) {
                MoveResult::In => level += 1,
                MoveResult::Out => {
                    if level == 1 {
                        break;
                    }
                    level -= 1;
                }
                _ => {}
            }
        }
    }
}

impl TitleCursor {
    /// 切分节点名。
    pub fn split_on<'de>(&self, dtb: RefDtb<'de>) -> (&'de str, BodyCursor) {
        let mut index = self.0 + 1;
        let mut len = 0;

        let structure = &dtb.borrow().structure;
        while let Some(block) = structure.get(index) {
            index += 1;
            if block.is_end_of_str() {
                let end = block.str_end();
                len += end;
                let s = structure[self.0 + 1].lead_str(len);
                return (s, AnyCursor(index, PhantomData));
            } else {
                len += 4;
            }
        }
        todo!()
    }

    /// 生成组光标。
    pub fn take_group_on(&self, dtb: RefDtb, name: &str) -> MultiNodeCursor {
        let name_bytes = name.as_bytes();
        let name_skip = align(name_bytes.len() + 1, BLOCK_LEN);
        let group = AnyCursor::<Body>(self.0, PhantomData);

        let title_body = AnyCursor::<Body>(self.0 + 1 + name_skip, PhantomData);
        let mut body = title_body;
        let mut len = 1;

        let structure = &dtb.borrow().structure;
        loop {
            body.skip_str_on(dtb);
            body.escape_from(dtb);
            if let Cursor::Title(c) = body.move_on(dtb) {
                let s = structure[c.0 + 1].lead_slice(name_bytes.len() + 1);
                if let [name @ .., b'@'] = s {
                    if name == name_bytes {
                        body.0 += 1 + name_skip;
                        len += 1;
                        continue;
                    }
                }
            }
            break;
        }
        MultiNodeCursor {
            start_cursor: group,
            skip_cursor: body,
            data_cursor: title_body,
            node_count: len,
        }
    }

    /// 生成节点光标。
    pub fn take_node_on(&self, dtb: RefDtb, name: &str) -> MultiNodeCursor {
        let name_bytes = name.as_bytes();
        let name_skip = align(name_bytes.len() + 1, BLOCK_LEN);
        let origin = AnyCursor::<Body>(self.0, PhantomData);
        let node = AnyCursor::<Body>(self.0 + 1 + name_skip, PhantomData);

        let mut body = AnyCursor::<Body>(self.0 + 1 + name_skip, PhantomData);

        body.escape_from(dtb);
        MultiNodeCursor {
            start_cursor: origin,
            skip_cursor: body,
            data_cursor: node,
            node_count: 1,
        }
    }
}

impl PropCursor {
    pub fn name_on<'a>(&self, dtb: RefDtb<'a>) -> (&'a str, BodyCursor) {
        let dtb = dtb.borrow();
        if let [_, len_data, off_name, ..] = &dtb.structure[self.0..] {
            use core::{slice, str};

            let off_name = off_name.as_usize();
            let s = &dtb.strings[off_name..];
            let len = s.iter().take_while(|b| **b != b'\0').count();
            (
                unsafe { str::from_utf8_unchecked(slice::from_raw_parts(s.as_ptr(), len)) },
                AnyCursor(
                    self.0 + 3 + align(len_data.as_usize(), BLOCK_LEN),
                    PhantomData,
                ),
            )
        } else {
            todo!()
        }
    }

    pub fn data_on<'a>(&self, dtb: RefDtb<'a>) -> &'a [u8] {
        if let [_, len_data, _, data @ ..] = &dtb.borrow().structure[self.0..] {
            data[0].lead_slice(len_data.as_usize())
        } else {
            todo!()
        }
    }

    pub fn map_on<T>(&self, dtb: RefDtb<'_>, f: impl FnOnce(&[u8]) -> T) -> T {
        if let [_, len_data, _, data @ ..] = &dtb.borrow().structure[self.0..] {
            f(data[0].lead_slice(len_data.as_usize()))
        } else {
            todo!()
        }
    }

    pub fn map_u32_on(&self, dtb: RefDtb<'_>) -> Result<u32, DtError> {
        let structure = &dtb.borrow().structure[self.0..];
        if let [_, len_data, _, data @ ..] = structure {
            if len_data.as_usize() == BLOCK_LEN {
                Ok(u32::from_be(data[0].0))
            } else {
                Err(DtError::buildin_type_parsed_error(
                    "u32",
                    self.file_index_on(dtb),
                ))
            }
        } else {
            Err(DtError::slice_eof_unpexpected(
                (4 * BLOCK_LEN) as _,
                (4 * structure.len()) as _,
                self.file_index_on(dtb),
            ))
        }
    }
}

#[derive(Debug)]
pub(super) enum Cursor {
    Title(TitleCursor),
    Prop(PropCursor),
    End,
}

impl Cursor {
    #[inline]
    const fn title(c: usize) -> Self {
        Self::Title(AnyCursor(c, PhantomData))
    }

    #[inline]
    const fn prop(c: usize) -> Self {
        Self::Prop(AnyCursor(c, PhantomData))
    }

    #[inline]
    const fn end() -> Self {
        Self::End
    }
}

#[inline]
const fn align(len: usize, align: usize) -> usize {
    len.div_ceil(align)
}
