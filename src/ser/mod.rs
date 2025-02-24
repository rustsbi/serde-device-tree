pub mod patch;
pub mod pointer;
pub mod serializer;
pub mod string_block;

use crate::common::*;
use crate::ser::patch::Patch;

/// We do run-twice on convert, first time to generate string block, second time todo real
/// structure.
pub fn to_dtb<'se, T>(data: &T, list: &'se [Patch<'se>], writer: &'se mut [u8]) -> Result<(), Error>
where
    T: serde::ser::Serialize,
{
    let mut offset: usize = 0;
    {
        let mut dst = crate::ser::pointer::Pointer::new(None);
        let mut patch_list = crate::ser::patch::PatchList::new(list);
        let mut block = crate::ser::string_block::StringBlock::new(writer, &mut offset);
        let mut ser =
            crate::ser::serializer::Serializer::new(&mut dst, &mut block, &mut patch_list);
        data.serialize(&mut ser)?;
    };
    list.iter().for_each(|patch| patch.init());
    {
        // Write from bottom to top, to avoid overlap.
        for i in (0..offset).rev() {
            writer[writer.len() - offset + i] = writer[i];
            writer[i] = 0;
        }
        // TODO: make sure no out of bound.

        // -1 for end zero.
        let (data_block, string_block) = writer.split_at_mut(writer.len() - offset);
        let (_, data_block) = data_block.split_at_mut(size_of::<crate::common::Header>());
        let mut patch_list = crate::ser::patch::PatchList::new(list);
        let mut block = crate::ser::string_block::StringBlock::new(string_block, &mut offset);
        let mut dst = crate::ser::pointer::Pointer::new(Some(data_block));
        let mut ser =
            crate::ser::serializer::Serializer::new(&mut dst, &mut block, &mut patch_list);
        ser.dst.step_by_u32(FDT_END);
        data.serialize(&mut ser)?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Unknown,
}

impl core::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:?}", self)
    }
}

impl core::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Unknown
    }
}
