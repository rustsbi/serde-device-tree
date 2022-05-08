use crate::{
    common::{Header, ALIGN},
    Error as DtError,
};
use core::{cell::RefCell, fmt::Display};

/// 设备树指针。
///
/// 用于构造设备树或跨虚存传递。
#[repr(transparent)]
pub struct DtbPtr(usize);

impl TryFrom<usize> for DtbPtr {
    type Error = DtError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::from_raw(value as _)
    }
}

impl DtbPtr {
    /// 验证指针指向的设备树，并构造 `DtbPtr`。
    pub fn from_raw(ptr: *mut u8) -> Result<Self, DtError> {
        let ptr = ptr as usize;
        if ptr & (ALIGN - 1) != 0 {
            Err(DtError::unaligned(ptr))
        } else {
            unsafe { &*(ptr as *const Header) }
                .verify()
                .map(|_| Self(ptr))
        }
    }

    /// 计算能容纳整个设备树的最小对齐。
    pub const fn align(&self) -> usize {
        let header = unsafe { &*(self.0 as *const Header) };
        let len = u32::from_be(header.total_size) as usize;
        let mut res = ALIGN;
        while res < len {
            res <<= 1;
        }
        res
    }
}

/// 对齐到 4 字节的设备树结构块。
#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub(super) struct StructureBlock(pub u32);

/// 结构块长度。
pub(super) const BLOCK_LEN: usize = core::mem::size_of::<StructureBlock>();

impl Display for StructureBlock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", u32::to_be_bytes(self.0))
    }
}

impl StructureBlock {
    /// 节点起始符。
    pub const NODE_BEGIN: Self = Self(1u32.to_be());
    /// 节点终止符。
    pub const NODE_END: Self = Self(2u32.to_be());
    /// 属性起始符。
    pub const PROP: Self = Self(3u32.to_be());
    /// 块占位符。
    pub const NOP: Self = Self(4u32.to_be());
    /// 结构区终止符。
    #[allow(unused)]
    pub const END: Self = Self(9u32.to_be());

    /// 一个 '\0' 结尾字符串结束于此块。
    pub const fn is_end_of_str(&self) -> bool {
        matches!(self.0.to_ne_bytes(), [_, _, _, 0])
    }

    /// '\0' 结尾字符串的实际结尾。
    pub fn str_end(&self) -> *const u8 {
        let remnant = match self.0.to_ne_bytes() {
            [0, _, _, _] => 0,
            [_, 0, _, _] => 1,
            [_, _, 0, _] => 2,
            [_, _, _, _] => 3,
        };
        unsafe { (self as *const _ as *const u8).add(remnant) }
    }

    /// 转换为描述字节长度或偏移的数值。
    pub const fn as_usize(&self) -> usize {
        u32::from_be(self.0) as _
    }

    /// 构造字节切片。
    ///
    /// TODO
    pub fn lead_slice<'a>(&self, len: usize) -> &'a [u8] {
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, len) }
    }
}

/// 设备树的映射形式。
pub struct Dtb {
    ptr: *const u8,
    pub(super) structure: &'static mut [StructureBlock],
    pub(super) strings: &'static [u8],
}

impl From<Dtb> for DtbPtr {
    fn from(dtb: Dtb) -> Self {
        Self(dtb.ptr as _)
    }
}

impl From<DtbPtr> for Dtb {
    fn from(ptr: DtbPtr) -> Self {
        let header = unsafe { &*(ptr.0 as *const Header) };

        let off_structure = u32::from_be(header.off_dt_struct);
        let len_structure = u32::from_be(header.size_dt_struct);
        let off_strings = u32::from_be(header.off_dt_strings);
        let len_strings = u32::from_be(header.size_dt_strings);

        let ptr_structure = off_structure as usize + ptr.0;
        let len_structure = len_structure as usize;
        let ptr_strings = off_strings as usize + ptr.0;
        let len_strings = len_strings as usize;

        unsafe {
            Self {
                ptr: ptr.0 as _,
                structure: core::slice::from_raw_parts_mut(
                    ptr_structure as *mut StructureBlock,
                    len_structure / core::mem::size_of::<StructureBlock>(),
                ),
                strings: core::slice::from_raw_parts(ptr_strings as _, len_strings),
            }
        }
    }
}

impl Dtb {
    /// 构造一个可安全共享的设备树映射。
    pub fn share(self) -> RefCell<Self> {
        RefCell::new(self)
    }

    /// 获取结构块的相对偏移。
    pub fn off_dt_struct(&self) -> usize {
        u32::from_be(unsafe { &*(self.ptr as *const Header) }.off_dt_struct) as _
    }
}

pub(super) type RefDtb<'a> = &'a RefCell<Dtb>;
