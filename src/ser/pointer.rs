use crate::common::*;

pub struct Pointer<'se> {
    offset: usize,
    data: Option<&'se mut [u8]>,
}

impl<'se> Pointer<'se> {
    #[inline(always)]
    pub fn new(dst: Option<&'se mut [u8]>) -> Pointer<'se> {
        Pointer {
            offset: 0,
            data: dst,
        }
    }

    #[inline(always)]
    pub fn update_data(&mut self, data: Option<&'se mut [u8]>) {
        self.data = data;
    }

    #[inline(always)]
    pub fn get_offset(&self) -> usize {
        self.offset
    }

    #[inline(always)]
    pub fn write_to_offset_u32(&mut self, offset: usize, value: u32) {
        match self.data {
            Some(ref mut data) => {
                data[offset..offset + 4].copy_from_slice(&u32::to_be_bytes(value))
            }
            None => {}
        }
    }

    #[inline(always)]
    pub fn step_by_prop(&mut self) -> usize {
        self.step_by_u32(FDT_PROP);
        let offset = self.offset;
        self.step_by_u32(FDT_NOP); // When create prop header, we do not know how long of the prop value.
        self.step_by_u32(FDT_NOP); // We can not assume this is a prop, so nop for default.
        offset
    }

    #[inline(always)]
    pub fn step_by_len(&mut self, len: usize) {
        self.offset += len
    }

    #[inline(always)]
    pub fn step_by_u32(&mut self, value: u32) {
        match self.data {
            Some(ref mut data) => {
                data[self.offset..self.offset + 4].copy_from_slice(&u32::to_be_bytes(value))
            }
            None => {}
        }
        self.step_by_len(4);
    }

    #[inline(always)]
    pub fn step_by_u8(&mut self, value: u8) {
        match self.data {
            Some(ref mut data) => data[self.offset] = value,
            None => {}
        }
        self.step_by_len(1);
    }

    #[inline(always)]
    pub fn step_align(&mut self) {
        while self.offset % 4 != 0 {
            match self.data {
                Some(ref mut data) => data[self.offset] = 0,
                None => {}
            }
            self.offset += 1;
        }
    }

    #[inline(always)]
    pub fn step_by_name(&mut self, name: &str) {
        name.bytes().for_each(|x| {
            self.step_by_u8(x);
        });
        self.step_by_u8(0);
        self.step_align();
    }
}
