use super::patch::{Patch, PatchList};
use super::pointer::Pointer;
use super::string_block::StringBlock;
use crate::common::*;
use crate::ser::Error;

// The enum for current parsing type.
#[derive(Clone, Copy, Debug)]
pub enum ValueType {
    Node,
    Prop,
}

/// SerializerInner
/// - `dst`: Pointer of distance &[u8] and the ref of &[u8].
pub struct SerializerInner<'se> {
    pub dst: &'se mut Pointer<'se>,
    string_block: &'se mut StringBlock<'se>,
    patch_list: &'se mut PatchList<'se>,
}

/// Serializer
/// - `ser`: all mutable reference of result.
pub struct Serializer<'a, 'se> {
    pub ser: &'a mut SerializerInner<'se>,

    prop_token_offset: usize,
    overwrite_patch: Option<&'se Patch<'se>>,
    current_name: &'se str,
    current_dep: usize,
}

impl<'se> SerializerInner<'se> {
    #[inline(always)]
    pub fn new(
        dst: &'se mut Pointer<'se>,
        string_block: &'se mut StringBlock<'se>,
        patch_list: &'se mut PatchList<'se>,
    ) -> Self {
        Self {
            dst,
            string_block,
            patch_list,
        }
    }
}

impl<'a, 'se> Serializer<'a, 'se> {
    #[inline(always)]
    pub fn new(inner: &'a mut SerializerInner<'se>) -> Self {
        Serializer {
            ser: inner,

            current_dep: 0,
            current_name: "",
            prop_token_offset: 0,
            overwrite_patch: None,
        }
    }

    #[inline(always)]
    pub fn get_next(self) -> Serializer<'a, 'se> {
        Serializer {
            ser: self.ser,
            current_dep: self.current_dep + 1,
            current_name: self.current_name,
            prop_token_offset: 0,
            overwrite_patch: None,
        }
    }

    #[inline(always)]
    pub fn get_next_ref<'b>(&'b mut self) -> Serializer<'b, 'se> {
        Serializer {
            ser: self.ser,
            current_dep: self.current_dep + 1,
            current_name: self.current_name,
            prop_token_offset: 0,
            overwrite_patch: None,
        }
    }
}

trait SerializeDynamicField<'se> {
    fn start_node(&mut self) -> Result<(), Error>;
    fn end_node(&mut self) -> Result<(), Error>;
    fn serialize_field_meta(&mut self, key: &'se str) -> Result<(), Error>;
    fn serialize_field_data<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: serde::ser::Serialize + ?Sized;
    fn serialize_dynamic_field<T>(&mut self, key: &'se str, value: &T) -> Result<(), Error>
    where
        T: serde::ser::Serialize + ?Sized;
}

impl<'se> SerializeDynamicField<'se> for Serializer<'_, 'se> {
    fn start_node(&mut self) -> Result<(), Error> {
        self.ser.dst.step_by_u32(FDT_BEGIN_NODE);
        if self.current_dep == 1 {
            // The name of root node should be empty.
            self.ser.dst.step_by_u32(0);
        } else {
            self.ser.dst.step_by_name(self.current_name);
        }
        self.ser.dst.step_align();

        Ok(())
    }
    fn end_node(&mut self) -> Result<(), Error> {
        for patch in self.ser.patch_list.add_list(self.current_dep) {
            let key = patch.get_depth_path(self.current_dep + 1);
            self.serialize_dynamic_field(key, patch.data)?;
        }
        self.ser.dst.step_by_u32(FDT_END_NODE);
        if self.current_dep == 1 {
            self.ser.dst.step_by_u32(FDT_END);
        }

        Ok(())
    }
    fn serialize_field_meta(&mut self, key: &'se str) -> Result<(), Error> {
        self.prop_token_offset = self.ser.dst.step_by_prop();
        self.current_name = key;
        self.overwrite_patch = self.ser.patch_list.step_forward(key, self.current_dep);

        Ok(())
    }
    fn serialize_field_data<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        let value_type = match self.overwrite_patch {
            Some(data) => {
                let ser = self.get_next_ref();
                data.serialize(ser);
                data.patch_type
            }
            None => {
                let ser = self.get_next_ref();
                value.serialize(ser)?.0
            }
        };

        // We now know how long the prop value.
        // TODO: make we have some better way than put nop, like move this block ahead.
        if let ValueType::Node = value_type {
            self.ser
                .dst
                .write_to_offset_u32(self.prop_token_offset - 4, FDT_NOP);
        } else {
            self.ser.dst.write_to_offset_u32(
                self.prop_token_offset,
                (self.ser.dst.get_offset() - self.prop_token_offset - 8) as u32,
            );
            self.ser.dst.write_to_offset_u32(
                self.prop_token_offset + 4,
                self.ser.string_block.find_or_insert(self.current_name) as u32,
            );
        }

        self.ser.dst.step_align();

        self.ser.patch_list.step_back(self.current_dep);

        Ok(())
    }
    fn serialize_dynamic_field<T>(&mut self, key: &'se str, value: &T) -> Result<(), Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        self.serialize_field_meta(key)?;
        self.serialize_field_data(value)?;
        Ok(())
    }
}

impl serde::ser::SerializeMap for Serializer<'_, '_> {
    type Ok = (ValueType, usize);
    type Error = Error;
    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: ?Sized + serde::ser::Serialize,
        V: ?Sized + serde::ser::Serialize,
    {
        if core::any::type_name::<K>() != "str" {
            panic!(
                "map key must be a str, but here is {}",
                core::any::type_name::<K>()
            );
        }
        let key = unsafe { *(core::ptr::addr_of!(key) as *const &str) };
        let mut ser = self.get_next_ref();
        ser.serialize_field_meta(key)?;
        ser.serialize_field_data(value)?;
        Ok(())
    }

    fn serialize_key<T>(&mut self, _input: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("only support serialize_entry")
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("only support serialize_entry")
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        self.end_node()?;
        Ok((ValueType::Node, self.ser.dst.get_offset()))
    }
}

impl serde::ser::SerializeStruct for Serializer<'_, '_> {
    type Ok = (ValueType, usize);
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        let mut ser = self.get_next_ref();
        ser.serialize_dynamic_field(key, value)?;

        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        self.end_node()?;
        Ok((ValueType::Node, self.ser.dst.get_offset()))
    }
}

impl serde::ser::SerializeStructVariant for Serializer<'_, '_> {
    type Ok = (ValueType, usize);
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

impl serde::ser::SerializeSeq for Serializer<'_, '_> {
    type Ok = (ValueType, usize);
    type Error = Error;
    // TODO: make sure there are no node seq serialize over this function.
    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        value.serialize(self.get_next_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Error> {
        // We think all seq we met is a prop.
        Ok((ValueType::Prop, self.ser.dst.get_offset()))
    }
}

impl serde::ser::SerializeTuple for Serializer<'_, '_> {
    type Ok = (ValueType, usize);
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::ser::Serialize,
    {
        value.serialize(self.get_next_ref())?;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Error> {
        Ok((ValueType::Prop, self.ser.dst.get_offset()))
    }
}

impl serde::ser::SerializeTupleVariant for Serializer<'_, '_> {
    type Ok = (ValueType, usize);
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("tuple_variant_field");
    }

    fn end(self) -> Result<Self::Ok, Error> {
        todo!("tuple_variant_end");
    }
}

impl serde::ser::SerializeTupleStruct for Serializer<'_, '_> {
    type Ok = (ValueType, usize);
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        todo!("tuple_struct_field");
    }

    fn end(self) -> Result<Self::Ok, Error> {
        todo!("tuple_struct_end");
    }
}

impl<'se> serde::ser::Serializer for Serializer<'_, 'se> {
    type Ok = (ValueType, usize);
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
        self.ser.dst.step_by_u32(v);
        Ok((ValueType::Prop, self.ser.dst.get_offset()))
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
        v.bytes().for_each(|x| {
            self.ser.dst.step_by_u8(x);
        });
        self.ser.dst.step_by_u8(0);
        Ok((ValueType::Prop, self.ser.dst.get_offset()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        v.iter().for_each(|x| self.ser.dst.step_by_u8(*x));
        Ok((ValueType::Prop, self.ser.dst.get_offset()))
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

    fn serialize_newtype_struct<T>(self, name: &'static str, v: &T) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        use crate::de_mut::node::{Node, NodeItem};
        use crate::de_mut::{NODE_NAME, NODE_NODE_ITEM_NAME};
        use core::ptr::addr_of;
        match name {
            NODE_NODE_ITEM_NAME => {
                let v = unsafe { &*(addr_of!(v) as *const &NodeItem<'se>) };
                self.serialize_newtype_struct(NODE_NAME, &v.deserialize::<Node>())
            }
            _ => todo!(),
        }
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
        let mut ser = self.get_next();
        ser.start_node()?;
        Ok(ser)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        let mut ser = self.get_next();
        ser.start_node()?;
        Ok(ser)
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

    #[cfg(not(feature = "std"))]
    fn collect_str<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + core::fmt::Display,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    const MAX_SIZE: usize = 256 + 32;
    #[test]
    fn base_ser_test() {
        #[derive(Serialize)]
        struct Base {
            pub hello: u32,
        }
        let mut buf1 = [0u8; MAX_SIZE];

        {
            let base = Base { hello: 0xdeedbeef };
            crate::ser::to_dtb(&base, &[], &mut buf1).unwrap();
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

        {
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 { hello: 0x10000001 },
            };
            crate::ser::to_dtb(&base, &[], &mut buf1).unwrap();
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

        {
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
            };
            crate::ser::to_dtb(&base, &[], &mut buf1).unwrap();
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

        {
            let base = Base {
                hello: 0xdeedbeef,
                base1: ["Hello", "World!", "Again"],
            };
            crate::ser::to_dtb(&base, &[], &mut buf1).unwrap();
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

        {
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
                hello2: 0x11223344,
                base2: Base1 { hello: "Roger" },
            };
            crate::ser::to_dtb(&base, &[], &mut buf1).unwrap();
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

        {
            let number = 0x55667788u32;
            let patch = crate::ser::patch::Patch::new(
                "/hello",
                &number as _,
                crate::ser::serializer::ValueType::Prop,
            );
            let list = [patch];
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
                hello2: 0x11223344,
                base2: Base1 { hello: "Roger" },
            };
            crate::ser::to_dtb(&base, &list, &mut buf1).unwrap();
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

        {
            let new_base = Base1 {
                hello: "replacement",
            };
            let patch = crate::ser::patch::Patch::new(
                "/hello",
                &new_base as _,
                crate::ser::serializer::ValueType::Node,
            );
            let list = [patch];
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
                hello2: 0x11223344,
                base2: Base1 { hello: "Roger" },
            };
            crate::ser::to_dtb(&base, &list, &mut buf1).unwrap();
        }
        // TODO: check buf1 buf2
        // println!("{:x?} {:x?}", buf1, buf2);
        // assert!(false);
    }
    #[test]
    fn add_node_ser_test() {
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

        {
            let new_base = Base1 { hello: "added" };
            let patch = crate::ser::patch::Patch::new(
                "/base3",
                &new_base as _,
                crate::ser::serializer::ValueType::Node,
            );
            let list = [patch];
            let base = Base {
                hello: 0xdeedbeef,
                base1: Base1 {
                    hello: "Hello, World!",
                },
                hello2: 0x11223344,
                base2: Base1 { hello: "Roger" },
            };
            crate::ser::to_dtb(&base, &list, &mut buf1).unwrap();
        }
        // TODO: check buf1 buf2
        // println!("{:x?}", buf1);
        // assert!(false);
    }
}
