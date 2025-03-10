use super::patch::PatchList;
use super::pointer::Pointer;
use super::string_block::StringBlock;
use crate::common::*;
use crate::ser::Error;

#[derive(Clone, Copy)]
// The enum for current parsing type.
enum ValueType {
    Node,
    Prop,
}

/// Serializer
/// - `dst`: Pointer of distance &[u8] and the ref of &[u8].
/// - `current_value_type`, `current_name`, `current_dep`: For recursive.
pub struct Serializer<'se> {
    pub dst: &'se mut Pointer<'se>,
    string_block: &'se mut StringBlock<'se>,
    patch_list: &'se mut PatchList<'se>,

    current_value_type: ValueType,
    current_name: &'se str,
    current_dep: usize,
}

impl<'se> Serializer<'se> {
    #[inline(always)]
    pub fn new(
        dst: &'se mut Pointer<'se>,
        cache: &'se mut StringBlock<'se>,
        patch_list: &'se mut PatchList<'se>,
    ) -> Serializer<'se> {
        Serializer {
            dst,
            string_block: cache,
            current_dep: 0,
            current_name: "",
            current_value_type: ValueType::Node,
            patch_list,
        }
    }
}

trait SerializeDynamicField<'se> {
    fn serialize_dynamic_field<T>(&mut self, key: &'se str, value: &T) -> Result<(), Error>
    where
        T: serde::ser::Serialize + ?Sized;
}

impl<'se> SerializeDynamicField<'se> for &mut Serializer<'se> {
    fn serialize_dynamic_field<T>(&mut self, key: &'se str, value: &T) -> Result<(), Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        let prop_header_offset = self.dst.step_by_prop();

        // Save prev
        let prev_type = self.current_value_type;
        let prev_name = self.current_name;
        self.current_dep += 1;
        self.current_name = key;
        let matched_patch = self.patch_list.step_forward(key, self.current_dep);

        match matched_patch {
            Some(data) => {
                data.serialize(self);
            }
            None => {
                value.serialize(&mut **self)?;
            }
        }

        // We now know how long the prop value.
        // TODO: make we have some better way than put nop, like move this block ahead.
        if let ValueType::Node = self.current_value_type {
            self.dst
                .write_to_offset_u32(prop_header_offset - 4, FDT_NOP);
        } else {
            self.dst.write_to_offset_u32(
                prop_header_offset,
                (self.dst.get_offset() - prop_header_offset - 8) as u32,
            );
            self.dst.write_to_offset_u32(
                prop_header_offset + 4,
                self.string_block.find_or_insert(key) as u32,
            );
        }

        self.dst.step_align();

        // Load prev
        self.patch_list.step_back(self.current_dep);
        self.current_value_type = prev_type;
        self.current_name = prev_name;
        self.current_dep -= 1;

        Ok(())
    }
}

impl serde::ser::SerializeMap for &mut Serializer<'_> {
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

impl serde::ser::SerializeStruct for &mut Serializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        self.serialize_dynamic_field(key, value)?;

        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok, Self::Error> {
        for patch in self.patch_list.add_list(self.current_dep) {
            let key = patch.get_depth_path(self.current_dep + 1);
            self.serialize_dynamic_field(key, patch.data)?;
        }
        self.dst.step_by_u32(FDT_END_NODE);
        Ok(())
    }
}

impl serde::ser::SerializeStructVariant for &mut Serializer<'_> {
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

impl serde::ser::SerializeSeq for &mut Serializer<'_> {
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

impl serde::ser::SerializeTuple for &mut Serializer<'_> {
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

impl serde::ser::SerializeTupleVariant for &mut Serializer<'_> {
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

impl serde::ser::SerializeTupleStruct for &mut Serializer<'_> {
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

impl<'se> serde::ser::Serializer for &mut Serializer<'se> {
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
        self.current_value_type = ValueType::Prop;
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
        self.current_value_type = ValueType::Prop;
        v.bytes().for_each(|x| {
            self.dst.step_by_u8(x);
        });
        self.dst.step_by_u8(0);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.current_value_type = ValueType::Prop;
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
        mut self,
        name: &'static str,
        v: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: serde::ser::Serialize + ?Sized,
    {
        use crate::de_mut::node::{Node, NodeItem};
        use crate::de_mut::{NODE_NAME, NODE_NODE_ITEM_NAME};
        use core::ptr::addr_of;
        match name {
            NODE_NAME => {
                // TODO: match level
                self.current_value_type = ValueType::Node;
                let v = unsafe { &*(addr_of!(v) as *const &Node<'se>) };
                self.dst.step_by_u32(FDT_BEGIN_NODE);
                if self.current_dep == 0 {
                    // The name of root node should be empty.
                    self.dst.step_by_u32(0);
                } else {
                    self.dst.step_by_name(v.name());
                    self.dst.step_align();
                }
                for prop in v.props() {
                    self.serialize_dynamic_field(prop.get_name(), &prop)?;
                }
                for node in v.nodes() {
                    self.serialize_dynamic_field(
                        node.get_full_name(),
                        &node.deserialize::<Node>(),
                    )?;
                }
                self.dst.step_by_u32(FDT_END_NODE);
                Ok(())
            }
            NODE_NODE_ITEM_NAME => {
                self.current_value_type = ValueType::Node;
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
        self.current_value_type = ValueType::Prop;
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
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.dst.step_by_u32(FDT_BEGIN_NODE);
        if self.current_dep == 0 {
            // The name of root node should be empty.
            self.dst.step_by_u32(0);
        } else {
            self.dst.step_by_name(self.current_name);
        }
        self.dst.step_align();
        self.current_value_type = ValueType::Node;
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
            let patch = crate::ser::patch::Patch::new("/hello", &number as _);
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
            let patch = crate::ser::patch::Patch::new("/hello", &new_base as _);
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
            let patch = crate::ser::patch::Patch::new("/base3", &new_base as _);
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
