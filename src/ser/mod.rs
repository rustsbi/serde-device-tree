pub mod patch;
pub mod pointer;
pub mod serializer;
pub mod string_block;

use crate::common::*;
use crate::ser::patch::Patch;

// TODO: set reverse map
const RSVMAP_LEN: usize = 16;

/// Serialize the data to dtb, with a list fof Patch, write to the `writer`.
///
/// We do run-twice on convert, first time to generate string block, second time todo real
/// structure.
pub fn to_dtb<'se, T>(data: &T, list: &'se [Patch<'se>], writer: &'se mut [u8]) -> Result<(), Error>
where
    T: serde::ser::Serialize,
{
    writer.iter_mut().for_each(|x| *x = 0);

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
    // Write from bottom to top, to avoid overlap.
    for i in (0..offset).rev() {
        writer[writer.len() - offset + i] = writer[i];
        writer[i] = 0;
    }
    // TODO: make sure no out of bound.

    let writer_len = writer.len();
    let (data_block, string_block) = writer.split_at_mut(writer.len() - offset);
    let (header, data_block) = data_block.split_at_mut(HEADER_LEN as usize + RSVMAP_LEN);
    let struct_len;
    {
        let mut patch_list = crate::ser::patch::PatchList::new(list);
        let mut block = crate::ser::string_block::StringBlock::new(string_block, &mut offset);
        let mut dst = crate::ser::pointer::Pointer::new(Some(data_block));
        let mut ser =
            crate::ser::serializer::Serializer::new(&mut dst, &mut block, &mut patch_list);
        data.serialize(&mut ser)?;
        ser.dst.step_by_u32(FDT_END);
        struct_len = ser.dst.get_offset();
    }
    // Make header
    {
        let header = unsafe { &mut *(header.as_mut_ptr() as *mut Header) };
        header.magic = u32::from_be(DEVICE_TREE_MAGIC);
        header.total_size = u32::from_be(writer_len as u32);
        header.off_dt_struct = u32::from_be(HEADER_LEN + RSVMAP_LEN as u32);
        header.off_dt_strings = u32::from_be((writer_len - offset) as u32);
        header.off_mem_rsvmap = u32::from_be(HEADER_LEN);
        header.version = u32::from_be(SUPPORTED_VERSION);
        header.last_comp_version = u32::from_be(SUPPORTED_VERSION); // TODO: maybe 16
        header.boot_cpuid_phys = 0; // TODO: wtf is this prop
        header.size_dt_strings = u32::from_be(offset as u32);
        header.size_dt_struct = u32::from_be(struct_len as u32);
    }
    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Unknown,
}

impl core::fmt::Display for Error {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(formatter, "{:?}", self)
    }
}

impl core::error::Error for Error {}

impl serde::ser::Error for Error {
    fn custom<T>(_msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        Self::Unknown
    }
}
