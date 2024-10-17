use super::{BodyCursor, Cursor, PropCursor, RefDtb, RegConfig, StructDeserializer};
use core::marker::PhantomData;
use serde::{de, Deserialize};

#[allow(unused)]
#[derive(Clone)]
pub struct Node<'de> {
    dtb: RefDtb<'de>,
    cursor: BodyCursor,
    reg: RegConfig,
    props_start: Option<BodyCursor>,
    nodes_start: Option<BodyCursor>,
}

/// 节点迭代器。
pub struct NodeIter<'de, 'b> {
    node: &'b Node<'de>,
    cursor: BodyCursor,
    i: usize,
}

/// 节点对象。
pub struct NodeItem<'de> {
    dtb: RefDtb<'de>,
    reg: RegConfig,
    node: BodyCursor,
    name: &'de str,
}

/// 属性迭代器。
pub struct PropIter<'de, 'b> {
    node: &'b Node<'de>,
    cursor: BodyCursor,
    i: usize,
}

/// 属性对象。
#[allow(unused)]
pub struct PropItem<'de> {
    dtb: RefDtb<'de>,
    reg: RegConfig,
    prop: PropCursor,
    name: &'de str,
}

impl<'de> Node<'de> {
    pub unsafe fn covnert_from_struct_deseriallizer_pointer(ptr: *const u8) -> Self {
        let struct_deseriallizer = unsafe { &*(ptr as *const StructDeserializer) };
        println!("get node from {:?}", struct_deseriallizer.cursor);
        let dtb = struct_deseriallizer.dtb;
        let mut cursor = struct_deseriallizer.cursor;
        let mut prop: Option<BodyCursor> = None;
        let mut node: Option<BodyCursor> = None;
        loop {
            match cursor.move_on(dtb) {
                Cursor::Title(c) => {
                    let (name, _) = c.split_on(dtb);
                    let (_, next) = c.take_node_on(dtb, name);
                    if let None = node {
                        node = Some(cursor)
                    }
                    cursor = next;
                }
                Cursor::Prop(c) => {
                    let (_, next) = c.name_on(dtb);
                    if let None = prop {
                        prop = Some(cursor)
                    }
                    cursor = next;
                }
                Cursor::End => {
                    cursor.move_next(dtb);
                    break;
                }
            }
        }
        Node {
            cursor: struct_deseriallizer.cursor,
            reg: struct_deseriallizer.reg,
            dtb: struct_deseriallizer.dtb,
            props_start: prop,
            nodes_start: node,
        }
    }
    /// 获得节点迭代器。
    pub const fn node_iter<'b>(&'b self) -> Option<NodeIter<'de, 'b>> {
        match self.nodes_start {
            None => None,
            Some(node_cursor) => Some(NodeIter {
                node: self,
                cursor: node_cursor,
                i: 0,
            }),
        }
    }

    /// 获得属性迭代器。
    pub const fn prop_iter<'b>(&'b self) -> Option<PropIter<'de, 'b>> {
        match self.nodes_start {
            None => None,
            Some(node_cursor) => Some(PropIter {
                node: self,
                cursor: node_cursor,
                i: 0,
            }),
        }
    }
}

impl<'de, 'b> Iterator for NodeIter<'de, 'b> {
    type Item = NodeItem<'de>;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1;
        let dtb = self.node.dtb;
        if let Cursor::Title(c) = self.cursor.move_on(dtb) {
            let (name, _) = c.split_on(dtb);
            let (node_cursor, next) = c.take_node_on(dtb, name);
            let res = Some(Self::Item {
                dtb,
                reg: self.node.reg,
                node: node_cursor,
                name,
            });
            self.cursor = next;
            res
        } else {
            None
        }
    }
}

impl<'de, 'b> Iterator for PropIter<'de, 'b> {
    type Item = PropItem<'de>;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1;
        let dtb = self.node.dtb;
        if let Cursor::Prop(c) = self.cursor.move_on(dtb) {
            let (name, next) = c.name_on(dtb);
            let res = Some(Self::Item {
                dtb,
                reg: self.node.reg,
                prop: c,
                name,
            });
            self.cursor = next;
            res
        } else {
            None
        }
    }
}

impl<'de, 'b> Deserialize<'de> for Node<'b> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de, 'b> {
            marker: PhantomData<Node<'b>>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de, 'b> de::Visitor<'de> for Visitor<'de, 'b> {
            type Value = Node<'b>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "struct StrSeq")
            }

            fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                // 结构体转为内存切片，然后拷贝过来
                if v.len() == core::mem::size_of::<StructDeserializer>() {
                    Ok(unsafe {
                        Self::Value::covnert_from_struct_deseriallizer_pointer(v.as_ptr())
                    })
                } else {
                    Err(E::invalid_length(
                        v.len(),
                        &"`Node` is copied with wrong size.",
                    ))
                }
            }
        }

        serde::Deserializer::deserialize_newtype_struct(
            deserializer,
            "Node",
            Visitor {
                marker: PhantomData,
                lifetime: PhantomData,
            },
        )
    }
}

impl<'de> NodeItem<'de> {
    /// 反序列化一个节点的内容。
    pub fn deserialize<T: Deserialize<'de>>(&self) -> T {
        T::deserialize(&mut StructDeserializer {
            dtb: self.dtb,
            reg: self.reg,
            cursor: self.node,
        })
        .unwrap()
    }

    pub fn get_split_name(&self) -> (&str, &str) {
        let pre_len = self
            .name
            .as_bytes()
            .iter()
            .take_while(|b| **b != b'@')
            .count();
        let mut res = self.name.split_at(pre_len);
        // Remove @ prefix
        res.1 = res.1.split_at(1).1;
        res
    }

    pub fn get_full_name(&self) -> &str {
        self.name
    }
}

impl<'de> PropItem<'de> {
    pub fn get_name(&self) -> &str {
        self.name
    }
}
