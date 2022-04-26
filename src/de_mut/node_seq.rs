use super::*;
use core::marker::PhantomData;
use serde::{de, Deserialize};

#[allow(dead_code)]
#[derive(Debug)]
pub struct NodeSeq<T> {
    next: *mut u32,
    structure: *const u32,
    strings: &'static [u8],
    _phantom: PhantomData<T>,
}

pub(super) const NODE_INNER_IDENT: &str = "__NodeInner";

impl<'de, T> Deserialize<'de> for NodeSeq<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let NodeInner(structure, strings) = Deserialize::deserialize(deserializer)?;
        let mut sub_tree = DeviceTree::from_parts(structure, strings);
        while !sub_tree.end() {
            let marker = sub_tree.cursor;
            sub_tree.cursor += 1;
            sub_tree.skip_cstr().unwrap();
            sub_tree.skip_node().unwrap();
            sub_tree.structure[marker] = ((sub_tree.cursor - marker) as u32).into();
        }
        let strings = unsafe { core::slice::from_raw_parts(strings.as_ptr(), strings.len()) };
        Ok(Self {
            next: structure.as_ptr() as _,
            structure: structure.as_ptr() as _,
            strings,
            _phantom: PhantomData,
        })
    }
}

impl<T> NodeSeq<T> {
    pub fn at(&self) -> &str {
        unsafe {
            let mut ptr = self.next.add(1) as *const u8;
            while *ptr != b'@' {
                ptr = ptr.add(1);
            }
            ptr = ptr.add(1);
            let mut len = 0;
            while *ptr.add(len) != b'\0' {
                len += 1;
            }
            core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len))
        }
    }

    pub fn exist(&self) -> bool {
        let next = StructureBlock::from(unsafe { *self.next });
        match next.0 {
            OF_DT_BEGIN_NODE | OF_DT_END_NODE => false,
            _ => true,
        }
    }

    pub fn next(&mut self) {
        self.next = unsafe { self.next.add(*self.next as _) };
    }
}

impl<'de, T: Deserialize<'de>> NodeSeq<T> {
    pub fn deserialize(&self) -> Result<T, Error> {
        let structure = unsafe {
            let len = *self.next as usize;
            *self.next = u32::from_ne_bytes(OF_DT_BEGIN_NODE);
            let ptr = self.next as *const u8;
            core::slice::from_raw_parts(ptr, len * U32_LEN)
        };

        let mut device_tree = DeviceTree::from_parts(structure, self.strings);
        device_tree.cursor += 1;
        device_tree.skip_cstr().unwrap();

        let mut d = Deserializer {
            expect_strings: false,
            loaded: Tag::End,
            device_tree,
        };
        let result = T::deserialize(&mut d);
        unsafe { *self.next = (structure.len() / U32_LEN) as _ };
        result
    }
}

struct NodeInner<'a>(&'a [u8], &'a [u8]);

impl<'de: 'a, 'a> Deserialize<'de> for NodeInner<'a> {
    fn deserialize<D>(__deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de: 'a, 'a> {
            marker: PhantomData<NodeInner<'a>>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de: 'a, 'a> de::Visitor<'de> for Visitor<'de, 'a> {
            type Value = NodeInner<'a>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "tuple struct NodeInner")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let strings = seq.next_element()?.ok_or_else(|| {
                    de::Error::invalid_length(0usize, &"tuple struct NodeInner with 2 elements")
                })?;
                let structure = seq.next_element()?.ok_or_else(|| {
                    de::Error::invalid_length(1usize, &"tuple struct NodeInner with 2 elements")
                })?;
                Ok(NodeInner(structure, strings))
            }
        }

        serde::Deserializer::deserialize_tuple_struct(
            __deserializer,
            NODE_INNER_IDENT,
            2usize,
            Visitor {
                marker: PhantomData,
                lifetime: PhantomData,
            },
        )
    }
}
