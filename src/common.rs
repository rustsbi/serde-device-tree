use crate::error::Error;

#[derive(Debug, Clone)]
#[repr(C)]
pub(crate) struct Header {
    pub magic: u32,
    pub total_size: u32,
    pub off_dt_struct: u32,
    pub off_dt_strings: u32,
    pub off_mem_rsvmap: u32,
    pub version: u32,
    pub last_comp_version: u32,
    pub boot_cpuid_phys: u32,
    pub size_dt_strings: u32,
    pub size_dt_struct: u32,
}

const DEVICE_TREE_MAGIC: u32 = 0xD00DFEED;
const U32_LEN: u32 = core::mem::size_of::<u32>() as _;

pub(crate) const ALIGN: usize = core::mem::align_of::<usize>();
pub(crate) const HEADER_LEN: u32 = core::mem::size_of::<Header>() as _;
pub(crate) const FDT_BEGIN_NODE: u32 = 0x1;
pub(crate) const FDT_END_NODE: u32 = 0x2;
pub(crate) const FDT_PROP: u32 = 0x3;
pub(crate) const FDT_NOP: u32 = 0x4;
pub(crate) const FDT_END: u32 = 0x9;
pub(crate) const SUPPORTED_VERSION: u32 = 17;

impl Header {
    pub fn verify(&self) -> Result<(), Error> {
        let header_base = self as *const _ as usize;
        // ---
        let magic = u32::from_be(self.magic);
        if magic != DEVICE_TREE_MAGIC {
            return Err(Error::invalid_magic(magic));
        }
        // ---
        let last_comp_version = u32::from_be(self.last_comp_version);
        if last_comp_version > SUPPORTED_VERSION {
            let file_index = (&self.last_comp_version as *const _ as usize) - header_base;
            return Err(Error::incompatible_version(
                last_comp_version,
                SUPPORTED_VERSION,
                file_index,
            ));
        }
        // ---
        let total_size = u32::from_be(self.total_size);
        if total_size < HEADER_LEN {
            let file_index = (&self.total_size as *const _ as usize) - header_base;
            return Err(Error::header_too_short(total_size, HEADER_LEN, file_index));
        }
        // ---
        let off_dt_struct = u32::from_be(self.off_dt_struct);
        if off_dt_struct < HEADER_LEN {
            let file_index = (&self.off_dt_struct as *const _ as usize) - header_base;
            return Err(Error::structure_index_underflow(
                off_dt_struct,
                HEADER_LEN,
                file_index,
            ));
        }
        let size_dt_struct = u32::from_be(self.size_dt_struct);
        if off_dt_struct + size_dt_struct > total_size {
            let file_index = (&self.size_dt_struct as *const _ as usize) - header_base;
            return Err(Error::structure_index_overflow(
                off_dt_struct + size_dt_struct,
                HEADER_LEN,
                file_index,
            ));
        }
        // ---
        let dt_struct = unsafe {
            core::slice::from_raw_parts(
                (header_base + off_dt_struct as usize) as *const u32,
                (size_dt_struct / U32_LEN) as usize,
            )
        };
        if u32::from_be(dt_struct[0]) != FDT_BEGIN_NODE {
            let file_index = dt_struct.as_ptr() as usize - header_base;
            return Err(Error::invalid_tag_id(
                u32::from_be(dt_struct[0]),
                file_index,
            ));
        }
        if u32::from_be(dt_struct[1]) != 0 {
            let file_index = dt_struct[1..].as_ptr() as usize - header_base;
            return Err(Error::invalid_tag_id(
                u32::from_be(dt_struct[1]),
                file_index,
            ));
        }
        let dt_struct_tail = &dt_struct[dt_struct.len() - 2..];
        if u32::from_be(dt_struct_tail[0]) != FDT_END_NODE {
            let file_index = dt_struct_tail.as_ptr() as usize - header_base;
            return Err(Error::invalid_tag_id(
                u32::from_be(dt_struct_tail[0]),
                file_index,
            ));
        }
        if u32::from_be(dt_struct_tail[1]) != FDT_END {
            let file_index = dt_struct_tail[1..].as_ptr() as usize - header_base;
            return Err(Error::invalid_tag_id(
                u32::from_be(dt_struct_tail[1]),
                file_index,
            ));
        }
        // ---
        let off_dt_strings = u32::from_be(self.off_dt_strings);
        if off_dt_strings < HEADER_LEN {
            let file_index = (&self.off_dt_strings as *const _ as usize) - header_base;
            return Err(Error::string_index_underflow(
                off_dt_strings,
                HEADER_LEN,
                file_index,
            ));
        }
        let size_dt_strings = u32::from_be(self.size_dt_strings);
        if off_dt_struct + size_dt_strings > total_size {
            let file_index = (&self.size_dt_strings as *const _ as usize) - header_base;
            return Err(Error::string_index_overflow(
                off_dt_strings,
                HEADER_LEN,
                file_index,
            ));
        }
        // ---
        let off_mem_rsvmap = u32::from_be(self.off_mem_rsvmap);
        if off_mem_rsvmap < HEADER_LEN {
            let file_index = (&self.off_mem_rsvmap as *const _ as usize) - header_base;
            return Err(Error::mem_rsvmap_index_underflow(
                off_mem_rsvmap,
                HEADER_LEN,
                file_index,
            ));
        }
        Ok(())
    }
}
