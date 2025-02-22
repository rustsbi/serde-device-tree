pub mod patch;
pub mod pointer;
pub mod string_block;

use crate::common::*;
use patch::PatchList;
use pointer::Pointer;
use string_block::StringBlock;

#[derive(Clone, Copy)]
pub enum ValueType {
    Node,
    Prop,
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

pub struct Serializer<'de> {
    dst: &'de mut Pointer<'de>,
    string_block: &'de mut StringBlock<'de>,
    value_type: ValueType,
    current_dep: usize,
    patch_list: &'de mut PatchList<'de>,
}

impl<'de> Serializer<'de> {
    pub fn new(
        dst: &'de mut Pointer<'de>,
        cache: &'de mut StringBlock<'de>,
        patch_list: &'de mut PatchList<'de>,
    ) -> Serializer<'de> {
        Serializer {
            dst,
            string_block: cache,
            current_dep: 0,
            value_type: ValueType::Node,
            patch_list,
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
        self.current_dep += 1;
        let matched_patch = self.patch_list.step_forward(key, self.current_dep);

        match matched_patch {
            Some(data) => {
                data.serialize(&mut **self);
            }
            None => {
                value.serialize(&mut **self)?;
            }
        }

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
        self.patch_list.step_back(self.current_dep);
        self.current_dep -= 1;

        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        // TODO: patch type add
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

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.value_type = ValueType::Prop;
        v.iter().for_each(|x| self.dst.step_by_u8(*x));
        Ok(())
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
        if self.current_dep == 0 {
            // The name of root node should be empty.
            self.dst.step_by_u32(0);
        } else {
            self.dst.step_by_name(name);
        }
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
    const MAX_SIZE: usize = 128 + 64;
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
            let mut patch_list = crate::ser::PatchList::new(&mut []);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block, &mut patch_list);
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
            let mut patch_list = crate::ser::PatchList::new(&mut []);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block, &mut patch_list);
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
            let mut patch_list = crate::ser::PatchList::new(&mut []);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block, &mut patch_list);
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
            let mut patch_list = crate::ser::PatchList::new(&mut []);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block, &mut patch_list);
            let base = Base {
                hello: 0xdeedbeef,
                base1: ["Hello", "World!", "Again"],
            };
            base.serialize(&mut ser).unwrap();
            ser.dst.step_by_u32(FDT_END);
        }
        // TODO: check buf1 buf2
        // println!("{:x?} {:x?}", buf1, buf2);
        // assert!(false);
    }
    #[test]
    fn node_prop_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: u32,
            pub base1: Base1,
            pub hello2: u32,
            pub base2: Base1,
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
            let mut patch_list = crate::ser::PatchList::new(&mut []);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block, &mut patch_list);
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
                hello2: 0x11223344,
                base2: Base1 { hello: "Roger" },
            };
            base.serialize(&mut ser).unwrap();
            ser.dst.step_by_u32(FDT_END);
        }
        // TODO: check buf1 buf2
        // println!("{:x?} {:x?}", buf1, buf2);
        // assert!(false);
    }
    #[test]
    fn replace_prop_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: u32,
            pub base1: Base1,
            pub hello2: u32,
            pub base2: Base1,
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
            let number = 0x55667788u32;
            let patch = crate::ser::patch::Patch::new("/hello", &number as _);
            let mut list = [patch];
            let mut patch_list = crate::ser::PatchList::new(&mut list);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block, &mut patch_list);
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
                hello2: 0x11223344,
                base2: Base1 { hello: "Roger" },
            };
            base.serialize(&mut ser).unwrap();
            ser.dst.step_by_u32(FDT_END);
        }
        // TODO: check buf1 buf2
        // println!("{:x?} {:x?}", buf1, buf2);
        // assert!(false);
    }
    #[test]
    fn replace_node_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: u32,
            pub base1: Base1,
            pub hello2: u32,
            pub base2: Base1,
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
            let new_base = Base1 {
                hello: "replacement",
            };
            let patch = crate::ser::patch::Patch::new("/hello", &new_base as _);
            let mut list = [patch];
            let mut patch_list = crate::ser::PatchList::new(&mut list);
            let mut ser = crate::ser::Serializer::new(&mut dst, &mut block, &mut patch_list);
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
                hello2: 0x11223344,
                base2: Base1 { hello: "Roger" },
            };
            base.serialize(&mut ser).unwrap();
            ser.dst.step_by_u32(FDT_END);
        }
        // TODO: check buf1 buf2
        println!("{:x?} {:x?}", buf1, buf2);
        assert!(false);
    }
}
