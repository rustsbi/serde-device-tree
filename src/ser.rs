use crate::common::*;

pub struct Pointer<'de> {
    pub offset: usize,
    pub data: &'de mut [u8],
}

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

#[derive(Clone, Copy)]
pub enum ValueType {
    Node,
    Prop,
}

pub struct Serializer<'de> {
    dst: &'de mut Pointer<'de>,
    string_block: &'de mut StringBlock<'de>,
    value_type: ValueType,
}

impl<'de> Serializer<'de> {
    pub fn new(dst: &'de mut Pointer<'de>, cache: &'de mut StringBlock<'de>) -> Serializer<'de> {
        Serializer {
            dst,
            string_block: cache,
            value_type: ValueType::Node,
        }
    }
}

impl<'a, 'de> serde::ser::SerializeMap for &'a mut Serializer<'de> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, _input: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("map_key");
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("map_value");
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!("map_end");
    }
}

impl<'a, 'de> serde::ser::SerializeStruct for &'a mut Serializer<'de> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        let prop_header_offset = self.dst.step_by_prop();
        let old_value_type = self.value_type;
        value.serialize(&mut **self)?;
        // We now know how long the prop value.
        // TODO: make we have some better way than put nop, like move this block ahead.
        if let ValueType::Node = self.value_type {
            self.dst
                .write_to_offset_u32(prop_header_offset - 4, FDT_NOP);
        } else {
            self.dst.write_to_offset_u32(
                prop_header_offset,
                (self.dst.offset - prop_header_offset - 8) as u32,
            );
            self.dst.write_to_offset_u32(
                prop_header_offset + 4,
                self.string_block.find_or_insert(key) as u32,
            );
        }
        self.value_type = old_value_type;
        self.dst.step_align();
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.dst.step_by_u32(FDT_END_NODE);
        Ok(())
    }
}

impl<'a, 'de> serde::ser::SerializeStructVariant for &'a mut Serializer<'de> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("struct_field");
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        todo!("struct_end");
    }
}

impl<'a, 'de> serde::ser::SerializeSeq for &'a mut Serializer<'de> {
    type Ok = ();
    type Error = Error;
    // TODO: make sure there are no node seq serialize over this function.
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    // Close the sequence.
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<'a, 'de> serde::ser::SerializeTuple for &'a mut Serializer<'de> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    // Close the sequence.
    fn end(self) -> Result<(), Error> {
        Ok(())
    }
}

impl<'a, 'de> serde::ser::SerializeTupleVariant for &'a mut Serializer<'de> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("tuple_variant_field");
    }

    fn end(self) -> Result<(), Error> {
        todo!("tuple_variant_end");
    }
}

impl<'a, 'de> serde::ser::SerializeTupleStruct for &'a mut Serializer<'de> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("tuple_struct_field");
    }

    fn end(self) -> Result<(), Error> {
        todo!("tuple_struct_end");
    }
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

impl<'a, 'de> serde::ser::Serializer for &'a mut Serializer<'de> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        todo!("bool");
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        todo!("i8");
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        todo!("i16");
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        todo!("i32");
    }

    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        todo!("i64");
    }

    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        todo!("u8");
    }

    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        todo!("u16");
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.value_type = ValueType::Prop;
        self.dst.step_by_u32(v);
        Ok(())
    }

    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        todo!("u64");
    }

    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        todo!("f32");
    }

    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        todo!("f64");
    }

    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        todo!("char");
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.value_type = ValueType::Prop;
        v.bytes().for_each(|x| {
            self.dst.step_by_u8(x);
        });
        self.dst.step_by_u8(0);
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        todo!("bytes");
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        todo!("none");
    }

    fn serialize_some<T>(self, _v: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("some");
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        todo!("unit");
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        todo!("unit struct");
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        todo!("unit struct variant");
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _v: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("newtype struct");
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("newtype struct variant");
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.value_type = ValueType::Prop;
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        todo!("tuple struct");
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        todo!("tuple variant");
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        todo!("map");
    }

    fn serialize_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.dst.step_by_u32(FDT_BEGIN_NODE);
        self.dst.step_by_name(name);
        self.value_type = ValueType::Node;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        todo!("struct variant");
    }
}

#[cfg(test)]
mod tests {
    use crate::common::*;
    use serde::ser::Serialize;
    use serde_derive::Serialize;
    const MAX_SIZE: usize = 128;
    #[test]
    fn base_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: u32,
        }
        let mut buf1 = [0u8; MAX_SIZE];
        let mut buf2 = [0u8; MAX_SIZE];

        {
            let mut dst = crate::ser::Pointer::new(&mut buf1);
            let mut block = crate::ser::StringBlock::new(&mut buf2);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block);
            let base = Base { hello: 0xdeedbeef };
            base.serialize(&mut ser).unwrap();
            // TODO: write end, this should be write by other thing.
            ser.dst.step_by_u32(FDT_END);
        }
        // TODO: check buf1 buf2
    }
    #[test]
    fn rev_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: u32,
            pub base1: Base1,
        }
        #[derive(Serialize)]
        struct Base1 {
            pub hello: u32,
        }
        let mut buf1 = [0u8; MAX_SIZE];
        let mut buf2 = [0u8; MAX_SIZE];

        {
            let mut dst = crate::ser::Pointer::new(&mut buf1);
            let mut block = crate::ser::StringBlock::new(&mut buf2);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block);
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 { hello: 0x10000001 },
            };
            base.serialize(&mut ser).unwrap();
            ser.dst.step_by_u32(FDT_END);
        }
        // TODO: check buf1 buf2
        // println!("{:x?} {:x?}", buf1, buf2);
        // assert!(false);
    }
    #[test]
    fn rev_str_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: u32,
            pub base1: Base1,
        }
        #[derive(Serialize)]
        struct Base1 {
            pub hello: &'static str,
        }
        let mut buf1 = [0u8; MAX_SIZE];
        let mut buf2 = [0u8; MAX_SIZE];

        {
            let mut dst = crate::ser::Pointer::new(&mut buf1);
            let mut block = crate::ser::StringBlock::new(&mut buf2);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block);
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
            };
            base.serialize(&mut ser).unwrap();
            ser.dst.step_by_u32(FDT_END);
        }
        // TODO: check buf1 buf2
        // println!("{:x?} {:x?}", buf1, buf2);
        // assert!(false);
    }
    #[test]
    fn seq_str_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: u32,
            pub base1: [&'static str; 3],
        }
        let mut buf1 = [0u8; MAX_SIZE];
        let mut buf2 = [0u8; MAX_SIZE];

        {
            let mut dst = crate::ser::Pointer::new(&mut buf1);
            let mut block = crate::ser::StringBlock::new(&mut buf2);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block);
            let base = Base {
                hello: 0xdeedbeef,
                base1: ["Hello", "World!", "Again"],
            };
            base.serialize(&mut ser).unwrap();
            ser.dst.step_by_u32(FDT_END);
        }
        // TODO: check buf1 buf2
        println!("{:x?} {:x?}", buf1, buf2);
        assert!(false);
    }
}
