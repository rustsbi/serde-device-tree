use super::{BodyCursor, Cursor, PropCursor, RefDtb, RegConfig, ValueCursor, ValueDeserializer};
use core::fmt::Debug;
use core::marker::PhantomData;
use serde::de::MapAccess;
use serde::{de, Deserialize};

#[allow(unused)]
#[derive(Clone)]
pub struct Node<'de> {
    dtb: RefDtb<'de>,
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
    body: BodyCursor,
    prop: PropCursor,
    name: &'de str,
}

impl<'de> Node<'de> {
    // TODO: Maybe use BTreeMap when have alloc
    /// 获得节点迭代器。
    pub fn nodes<'b>(&'b self) -> Option<NodeIter<'de, 'b>> {
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
    pub fn props<'b>(&'b self) -> Option<PropIter<'de, 'b>> {
        match self.props_start {
            None => None,
            Some(node_cursor) => Some(PropIter {
                node: self,
                cursor: node_cursor,
                i: 0,
            }),
        }
    }
}

impl Debug for Node<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let props = self.props();
        write!(f, "Props: [")?;
        if let Some(s) = props {
            let mut first_written = true;
            for prop in s {
                if first_written {
                    write!(f, "\"{}\"", prop.get_name())?;
                    first_written = false;
                } else {
                    write!(f, ",\"{}\"", prop.get_name())?;
                }
            }
        }
        writeln!(f, "]")?;

        let children = self.nodes();
        write!(f, "Children: [")?;
        if let Some(s) = children {
            let mut first_written = true;
            for child in s {
                if first_written {
                    write!(f, "\"{}\"", child.get_full_name())?;
                    first_written = false;
                } else {
                    write!(f, ",\"{}\"", child.get_full_name())?;
                }
            }
        }
        writeln!(f, "]")?;

        Ok(())
    }
}

impl<'de> Iterator for NodeIter<'de, '_> {
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

impl<'de> Iterator for PropIter<'de, '_> {
    type Item = PropItem<'de>;

    fn next(&mut self) -> Option<Self::Item> {
        self.i += 1;
        let dtb = self.node.dtb;
        if let Cursor::Prop(c) = self.cursor.move_on(dtb) {
            let (name, next) = c.name_on(dtb);
            let res = Some(Self::Item {
                dtb,
                body: self.cursor,
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
                while let Some((key, value)) = access.next_entry::<&str, ValueDeserializer<'b>>()? {
                    dtb = Some(value.dtb);
                    reg = Some(value.reg);
                    if key == "/" {
                        continue;
                    }
                    match value.cursor {
                        ValueCursor::Prop(_) => {
                            if props_start.is_none() {
                                props_start = Some(value.body_cursor);
                            }
                        }
                        ValueCursor::Body(_) => {
                            if nodes_start.is_none() {
                                nodes_start = Some(value.body_cursor);
                            }
                        }
                    }
                }

                Ok(Node {
                    dtb: dtb.unwrap(),
                    reg: reg.unwrap(),
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
            body_cursor: self.node,
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
            body_cursor: self.body,
            cursor: ValueCursor::Prop(self.prop),
        })
        .unwrap()
    }
}
