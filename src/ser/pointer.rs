use crate::common::*;

pub struct Pointer<'de> {
    pub offset: usize,
    pub data: &'de mut [u8],
}

impl<'de> Pointer<'de> {
    pub fn new(dst: &'de mut [u8]) -> Pointer<'de> {
        Pointer {
            offset: 0,
            data: dst,
        }
    }

    pub fn write_to_offset_u32(&mut self, offset: usize, value: u32) {
        self.data[offset..offset + 4].copy_from_slice(&u32::to_be_bytes(value));
    }

    pub fn step_by_prop(&mut self) -> usize {
        self.step_by_u32(FDT_PROP);
        let offset = self.offset;
        self.step_by_u32(FDT_NOP); // When create prop header, we do not know how long of the prop value.
        self.step_by_u32(FDT_NOP); // We can not assume this is a prop, so nop for default.
        offset
    }

    pub fn step_by_len(&mut self, len: usize) {
        self.offset += len
    }

    pub fn step_by_u32(&mut self, value: u32) {
        self.data[self.offset..self.offset + 4].copy_from_slice(&u32::to_be_bytes(value));
        self.step_by_len(4);
    }

    pub fn step_by_u8(&mut self, value: u8) {
        self.data[self.offset] = value;
        self.step_by_len(1);
    }

    pub fn step_align(&mut self) {
        while self.offset % 4 != 0 {
            self.data[self.offset] = 0;
            self.offset += 1;
        }
    }

    pub fn step_by_name(&mut self, name: &str) {
        name.bytes().for_each(|x| {
            self.step_by_u8(x);
        });
        self.step_by_u8(0);
        self.step_align();
    }
}
