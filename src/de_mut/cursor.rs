use super::{DtError, RefDtb, StructureBlock, BLOCK_LEN};
use core::marker::PhantomData;

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(super) struct AnyCursor<T: Type = Body>(usize, PhantomData<T>);

pub(super) type BodyCursor = AnyCursor<Body>;
pub(super) type TitleCursor = AnyCursor<Title>;
pub(super) type GroupCursor = AnyCursor<Group>;
pub(super) type PropCursor = AnyCursor<Prop>;

pub(super) trait Type {}

#[derive(Clone, Copy, Debug)]
pub(super) struct Body {}
#[derive(Clone, Copy, Debug)]
pub(super) struct Title {}
#[derive(Clone, Copy, Debug)]
pub(super) struct Group {}
#[derive(Clone, Copy, Debug)]
pub(super) struct Prop {}

impl Type for Body {}
impl Type for Title {}
impl Type for Group {}
impl Type for Prop {}

pub enum MoveResult {
    In,
    Out,
    Others,
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
            _ => todo!(),
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

        let structure = &dtb.borrow().structure;
        let ptr = structure[index..].as_ptr() as *const u8;
        while let Some(block) = structure.get(index) {
            index += 1;
            if block.is_end_of_str() {
                let end = block.str_end();
                let s = unsafe { core::slice::from_raw_parts(ptr, end.offset_from(ptr) as _) };
                let s = unsafe { core::str::from_utf8_unchecked(s) };
                return (s, AnyCursor(index, PhantomData));
            }
        }
        todo!()
    }

    /// 生成组光标。
    pub fn take_group_on(&self, dtb: RefDtb, name: &str) -> (GroupCursor, usize, BodyCursor) {
        let name_bytes = name.as_bytes();
        let name_skip = align(name_bytes.len() + 1, BLOCK_LEN);
        let group = AnyCursor::<Group>(self.0, PhantomData);

        let mut body = AnyCursor::<Body>(self.0 + 1 + name_skip, PhantomData);
        let mut len = 1;

        let structure = &dtb.borrow().structure;
        loop {
            body.skip_str_on(dtb);
            body.escape_from(dtb);
            if let Cursor::Title(c) = body.move_on(dtb) {
                let s = unsafe {
                    core::slice::from_raw_parts(
                        structure[c.0 + 1..].as_ptr() as *const u8,
                        name_bytes.len() + 1,
                    )
                };
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
        (group, len, body)
    }

    /// 生成节点光标。
    pub fn take_node_on(&self, dtb: RefDtb, name: &str) -> (BodyCursor, BodyCursor) {
        let name_bytes = name.as_bytes();
        let name_skip = align(name_bytes.len() + 1, BLOCK_LEN);
        let node = AnyCursor::<Body>(self.0 + 1 + name_skip, PhantomData);

        let mut body = AnyCursor::<Body>(self.0 + 1 + name_skip, PhantomData);

        body.escape_from(dtb);
        (node, body)
    }
}

impl GroupCursor {
    /// 读取缓存的下一项偏移。
    pub fn offset_on(&self, dtb: RefDtb) -> usize {
        (dtb.borrow().structure[self.0].0 >> 8) as _
    }

    /// 利用缓存的名字长度取出名字。
    pub fn name_on<'a>(&self, dtb: RefDtb<'a>) -> (&'a [u8], BodyCursor) {
        let structure = &dtb.borrow().structure;
        let len_name = (structure[self.0].0 & 0xff) as usize;
        let bytes = structure[self.0 + 1].lead_slice(len_name);
        (
            bytes,
            AnyCursor(self.0 + 1 + align(len_name + 1, BLOCK_LEN), PhantomData),
        )
    }

    /// 初始化组反序列化。
    pub fn init_on(&self, dtb: RefDtb, len_item: usize, len_name: usize) {
        let mut body = AnyCursor::<Body>(self.0, PhantomData);
        for _ in 0..len_item {
            let current = body.0;
            let len_total = dtb.borrow().structure[current + 1]
                .lead_slice(u16::MAX as _)
                .iter()
                .enumerate()
                .skip(len_name + 1)
                .find(|(_, b)| **b == b'\0')
                .map(|(i, _)| i)
                .unwrap();
            body.step_n(align(len_total, BLOCK_LEN));
            body.skip_str_on(dtb);
            body.escape_from(dtb);
            let off_next = body.0 - current;
            dtb.borrow_mut().structure[current].0 = (off_next << 8 | len_total) as _;
        }
    }

    /// 组结构恢复原状。
    pub fn drop_on(&self, dtb: RefDtb, len_item: usize) {
        use StructureBlock as B;
        let structure = &mut *dtb.borrow_mut().structure;
        let mut i = self.0;
        for _ in 0..len_item {
            let offset = (structure[i].0 >> 8) as usize;
            structure[i] = B::NODE_BEGIN;
            i += offset;
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
            unsafe { core::slice::from_raw_parts(data.as_ptr() as _, len_data.as_usize()) }
        } else {
            todo!()
        }
    }

    pub fn map_on<T>(&self, dtb: RefDtb<'_>, f: impl FnOnce(&[u8]) -> T) -> T {
        if let [_, len_data, _, data @ ..] = &dtb.borrow().structure[self.0..] {
            f(unsafe { core::slice::from_raw_parts(data.as_ptr() as _, len_data.as_usize()) })
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

    pub fn operate_on(&self, dtb: RefDtb<'_>, f: impl FnOnce(&mut [u8])) {
        if let [_, len_data, _, data @ ..] = &mut dtb.borrow_mut().structure[self.0..] {
            f(unsafe {
                core::slice::from_raw_parts_mut(data.as_mut_ptr() as _, len_data.as_usize())
            });
        } else {
            todo!()
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
    (len + align - 1) / align
}
