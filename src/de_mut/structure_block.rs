use super::U32_LEN;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub(super) struct StructureBlock(pub [u8; U32_LEN]);

impl From<u32> for StructureBlock {
    fn from(val: u32) -> Self {
        Self(u32::to_ne_bytes(val))
    }
}
