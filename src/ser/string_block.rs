pub struct StringBlock<'de> {
    pub end: usize,
    pub data: &'de mut [u8],
}

impl<'de> StringBlock<'de> {
    pub fn new(dst: &'de mut [u8]) -> StringBlock<'de> {
        StringBlock { data: dst, end: 0 }
    }

    /// Will panic when len > end
    /// TODO: show as error
    /// Return (Result String, End Offset)
    pub fn get_str_by_offset<'a>(&'a self, offset: usize) -> (&'a str, usize) {
        if offset > self.end {
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

    fn insert_u8(&mut self, data: u8) {
        self.data[self.end] = data;
        self.end += 1;
    }
    /// Return the start offset of inserted string.
    pub fn insert_str(&mut self, name: &str) -> usize {
        let result = self.end;
        name.bytes().for_each(|x| {
            self.insert_u8(x);
        });
        self.insert_u8(0);
        result
    }

    pub fn find_or_insert(&mut self, name: &str) -> usize {
        let mut current_pos = 0;
        while current_pos < self.end {
            let (result, new_pos) = self.get_str_by_offset(current_pos);
            if result == name {
                return current_pos;
            }
            current_pos = new_pos;
        }

        self.insert_str(name)
    }
}
