use super::{BodyCursor, Cursor, PropCursor, RefDtb, RegConfig, ValueCursor, ValueDeserializer};
use core::fmt::Debug;
use core::marker::PhantomData;
use serde::de::MapAccess;
use serde::{Deserialize, de};

// TODO: Spec 2.3.5 said that we should not inherited from ancestors and the size-cell &
// address-cells should only used for current node's children.
#[allow(unused)]
#[derive(Clone)]
pub struct Node<'de> {
    dtb: RefDtb<'de>,
    reg: RegConfig,
    cursor: BodyCursor,
    props_start: Option<BodyCursor>,
    nodes_start: Option<BodyCursor>,
}

/// 节点迭代器。
pub struct NodeIter<'de, 'b> {
    node: &'b Node<'de>,
    cursor: Option<BodyCursor>,
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
    cursor: Option<BodyCursor>,
    i: usize,
}

/// 属性对象。
#[allow(unused)]
pub struct PropItem<'de> {
    dtb: RefDtb<'de>,
    reg: RegConfig,
    body: BodyCursor,
    prop: PropCursor,
    name: &'de str,
}

impl<'de> Node<'de> {
    pub fn deserialize<T: Deserialize<'de>>(&self) -> T {
        use super::ValueCursor;
        T::deserialize(&mut ValueDeserializer {
            dtb: self.dtb,
            reg: self.reg,
            cursor: ValueCursor::Body(self.cursor),
        })
        .unwrap()
    }
    // TODO: Maybe use BTreeMap when have alloc
    /// 获得节点迭代器。
    pub fn nodes<'b>(&'b self) -> NodeIter<'de, 'b> {
        NodeIter {
            node: self,
            cursor: self.nodes_start,
            i: 0,
        }
    }

    /// 获得属性迭代器。
    pub fn props<'b>(&'b self) -> PropIter<'de, 'b> {
        PropIter {
            node: self,
            cursor: self.props_start,
            i: 0,
        }
    }

    /// 尝试获得指定属性
    pub fn get_prop<'b>(&'b self, name: &str) -> Option<PropItem<'b>> {
        self.props().find(|prop| prop.get_name() == name)
    }
}

impl Debug for Node<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let props = self.props();
        write!(f, "Props: [")?;
        let mut first_written = true;
        for prop in props {
            if first_written {
                write!(f, "\"{}\"", prop.get_name())?;
                first_written = false;
            } else {
                write!(f, ",\"{}\"", prop.get_name())?;
            }
        }
        writeln!(f, "]")?;

        let children = self.nodes();
        write!(f, "Children: [")?;
        let mut first_written = true;
        for child in children {
            if first_written {
                write!(f, "\"{}\"", child.get_full_name())?;
                first_written = false;
            } else {
                write!(f, ",\"{}\"", child.get_full_name())?;
            }
        }
        writeln!(f, "]")?;

        Ok(())
    }
}

impl<'de> Iterator for NodeIter<'de, '_> {
    type Item = NodeItem<'de>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut cursor) = self.cursor {
            self.i += 1;
            let dtb = self.node.dtb;
            if let Cursor::Title(c) = cursor.move_on(dtb) {
                let (name, _) = c.split_on(dtb);
                let node_cursor = c.take_node_on(dtb, name);
                let res = Some(Self::Item {
                    dtb,
                    reg: self.node.reg,
                    node: node_cursor.skip_cursor,
                    name,
                });
                *cursor = node_cursor.next_cursor;
                res
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<'de> Iterator for PropIter<'de, '_> {
    type Item = PropItem<'de>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut cursor) = self.cursor {
            self.i += 1;
            let dtb = self.node.dtb;
            if let Cursor::Prop(c) = cursor.move_on(dtb) {
                let (name, next) = c.name_on(dtb);
                let res = Some(Self::Item {
                    dtb,
                    body: *cursor,
                    reg: self.node.reg,
                    prop: c,
                    name,
                });
                *cursor = next;
                res
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<'de> Deserialize<'de> for Node<'_> {
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
                write!(formatter, "struct Node")
            }
            fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                // While there are entries remaining in the input, add them
                // into our map.
                let mut dtb: Option<RefDtb<'b>> = None;
                let mut reg: Option<RegConfig> = None;
                let mut props_start: Option<BodyCursor> = None;
                let mut nodes_start: Option<BodyCursor> = None;
                let mut self_cursor: Option<BodyCursor> = None;
                while let Some((key, value)) = access.next_entry::<&str, ValueDeserializer<'b>>()? {
                    dtb = Some(value.dtb);
                    reg = Some(value.reg);
                    if key == "/" {
                        self_cursor = match value.cursor {
                            ValueCursor::Body(cursor) => Some(cursor),
                            ValueCursor::Node(result) => Some(result.next_cursor),
                            _ => {
                                unreachable!("root of NodeSeq shouble be body cursor")
                            }
                        };
                        continue;
                    }
                    match value.cursor {
                        ValueCursor::Prop(cursor, _) => {
                            if props_start.is_none() {
                                props_start = Some(cursor);
                            }
                        }
                        ValueCursor::Node(cursor) => {
                            if nodes_start.is_none() {
                                nodes_start = Some(cursor.start_cursor);
                            }
                        }
                        _ => unreachable!("unparsed(body) cursor"),
                    }
                }

                Ok(Node {
                    dtb: dtb.unwrap(),
                    reg: reg.unwrap(),
                    cursor: self_cursor.unwrap(),
                    nodes_start,
                    props_start,
                })
            }
        }

        serde::Deserializer::deserialize_map(
            deserializer,
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
        T::deserialize(&mut ValueDeserializer {
            dtb: self.dtb,
            reg: self.reg,
            cursor: ValueCursor::Body(self.node),
        })
        .unwrap()
    }

    pub fn get_parsed_name(&self) -> (&str, Option<&str>) {
        if self.name.contains("@") {
            let pre_len = self
                .name
                .as_bytes()
                .iter()
                .take_while(|b| **b != b'@')
                .count();
            let (node_name, raw_unit_address) = self.name.split_at(pre_len);
            // Remove @ prefix
            let unit_address = raw_unit_address.split_at(1).1;
            (node_name, Some(unit_address))
        } else {
            (self.name, None)
        }
    }

    pub fn get_full_name(&self) -> &str {
        self.name
    }
}

impl<'de> PropItem<'de> {
    pub fn get_name(&self) -> &str {
        self.name
    }
    pub fn deserialize<T: Deserialize<'de>>(&self) -> T {
        use super::ValueCursor;
        T::deserialize(&mut ValueDeserializer {
            dtb: self.dtb,
            reg: self.reg,
            cursor: ValueCursor::Prop(self.body, self.prop),
        })
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::{Dtb, DtbPtr, buildin::Node, from_raw_mut};
    const RAW_DEVICE_TREE: &[u8] = include_bytes!("../../examples/hifive-unmatched-a00.dtb");
    const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();
    #[repr(align(8))]
    struct AlignedBuffer {
        pub data: [u8; RAW_DEVICE_TREE.len()],
    }
    #[test]
    fn test_find_prop() {
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Node = from_raw_mut(&dtb).unwrap();
        let prop = node.get_prop("compatible");
        assert!(prop.is_some());
    }
}
