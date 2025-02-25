/// StringBlock
/// As spec said, dtb have a block called string block for saving prop names.
pub struct StringBlock<'se> {
    end: &'se mut usize,
    data: &'se mut [u8],
}

impl<'se> StringBlock<'se> {
    /// Make a new string block.
    ///
    /// For get how long is string block, we make `end` as a mut ref.
    #[inline(always)]
    pub fn new(dst: &'se mut [u8], end: &'se mut usize) -> StringBlock<'se> {
        StringBlock { data: dst, end }
    }

    // TODO: show as error
    /// Assume the passing `offset` is the start of a string, and return this string.
    /// Return (Result String, End Offset).
    ///
    /// Will panic when len > end.
    #[inline(always)]
    pub fn get_str_by_offset(&self, offset: usize) -> (&str, usize) {
        if offset > *self.end {
            panic!("invalid read");
        }
        let current_slice = &self.data[offset..];
        let pos = current_slice
            .iter()
            .position(|&x| x == b'\0')
            .unwrap_or(self.data.len());
        let (a, _) = current_slice.split_at(pos + 1);
        let result = unsafe { core::str::from_utf8_unchecked(&a[..a.len() - 1]) };
        (result, pos + offset + 1)
    }

    #[inline(always)]
    fn insert_u8(&mut self, data: u8) {
        self.data[*self.end] = data;
        *self.end += 1;
    }

    /// Return the start offset of inserted string.
    #[inline(always)]
    pub fn insert_str(&mut self, name: &str) -> usize {
        let result = *self.end;
        name.bytes().for_each(|x| {
            self.insert_u8(x);
        });
        self.insert_u8(0);
        result
    }

    /// Find a string. If not found, insert it.
    #[inline(always)]
    pub fn find_or_insert(&mut self, name: &str) -> usize {
        let mut current_pos = 0;
        while current_pos < *self.end {
            let (result, new_pos) = self.get_str_by_offset(current_pos);
            if result == name {
                return current_pos;
            }
            current_pos = new_pos;
        }

        self.insert_str(name)
    }
}
