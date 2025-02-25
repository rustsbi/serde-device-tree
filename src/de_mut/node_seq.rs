use super::{BodyCursor, Cursor, RefDtb, RegConfig, ValueCursor, ValueDeserializer};
use core::{fmt::Debug, marker::PhantomData};
use serde::de::SeqAccess;
use serde::{Deserialize, de};

/// 一组名字以 `@...` 区分，同类、同级且连续的节点的映射。
///
/// 在解析前，无法得知这种节点的数量，因此也无法为它们分配足够的空间，
/// 因此这些节点将延迟解析。
/// 迭代 `NodeSeq` 可获得一系列 [`NodeSeqItem`]，再调用 `deserialize` 方法分别解析每个节点。
pub struct NodeSeq<'de> {
    name: &'de str,
    count: usize,
    starter: ValueDeserializer<'de>,
}

/// 连续节点迭代器。
pub struct NodeSeqIter<'de, 'b> {
    seq: &'b NodeSeq<'de>,
    de: ValueDeserializer<'de>,
    i: usize,
}

/// 连续节点对象。
pub struct NodeSeqItem<'de> {
    dtb: RefDtb<'de>,
    reg: RegConfig,
    body: BodyCursor,
    at: &'de str,
}

impl<'de> Deserialize<'de> for NodeSeq<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'de, 'b> {
            marker: PhantomData<NodeSeq<'b>>,
            lifetime: PhantomData<&'de ()>,
        }
        impl<'de, 'b> de::Visitor<'de> for Visitor<'de, 'b> {
            type Value = NodeSeq<'b>;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                write!(formatter, "struct ValueDeserializer")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut starter: Option<ValueDeserializer> = None;
                let mut count = 0;
                while let Some(node) = seq.next_element()? {
                    if starter.is_none() {
                        starter = Some(node);
                    }
                    count += 1
                }
                let mut starter = starter.unwrap();

                match starter.move_on() {
                    Cursor::Title(c) => {
                        let (name, _) = c.split_on(starter.dtb);

                        let (name, _) = name.split_once('@').unwrap_or((name, ""));
                        Ok(NodeSeq {
                            name,
                            count,
                            starter,
                        })
                    }
                    _ => unreachable!("NodeSeq should be inited by a node"),
                }
            }
        }

        serde::Deserializer::deserialize_seq(
            deserializer,
            Visitor {
                marker: PhantomData,
                lifetime: PhantomData,
            },
        )
    }
}

impl<'de> NodeSeq<'de> {
    /// 连续节点总数。
    pub const fn len(&self) -> usize {
        self.count
    }

    /// 如果连续节点数量为零，返回 true。但连续节点数量不可能为零。
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// 获得节点迭代器。
    pub const fn iter<'b>(&'b self) -> NodeSeqIter<'de, 'b> {
        NodeSeqIter {
            seq: self,
            de: self.starter,
            i: 0,
        }
    }
}

impl Debug for NodeSeq<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iter = self.iter();
        if let Some(s) = iter.next() {
            write!(f, "[@{}", s.at)?;
            for s in iter {
                write!(f, ", @{}", s.at)?;
            }
            write!(f, "]")
        } else {
            unreachable!("NodeSeq contains at least one node.")
        }
    }
}

impl<'de> Iterator for NodeSeqIter<'de, '_> {
    type Item = NodeSeqItem<'de>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.seq.len() {
            None
        } else {
            self.i += 1;
            match self.de.move_on() {
                // 子节点名字
                Cursor::Title(c) => {
                    let (full_name, _) = c.split_on(self.de.dtb);
                    let node_reuslt = c.take_node_on(self.de.dtb, full_name);

                    let (pre_name, suf_name) = full_name.split_once('@').unwrap_or((full_name, ""));
                    if self.seq.name != pre_name {
                        return None;
                    }

                    self.de.cursor = ValueCursor::Body(node_reuslt.skip_cursor);

                    Some(Self::Item {
                        dtb: self.de.dtb,
                        reg: self.de.reg,
                        body: node_reuslt.data_cursor,
                        at: suf_name,
                    })
                }
                _ => None,
            }
        }
    }
}

impl NodeSeqItem<'_> {
    /// 获得区分节点的序号。
    pub fn at(&self) -> &str {
        self.at
    }
}

impl<'de> NodeSeqItem<'de> {
    /// 反序列化一个节点的内容。
    pub fn deserialize<T: Deserialize<'de>>(&self) -> T {
        T::deserialize(&mut ValueDeserializer {
            dtb: self.dtb,
            reg: self.reg,
            cursor: ValueCursor::Body(self.body),
        })
        .unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::buildin::{NodeSeq, Reg};
    use crate::{Dtb, DtbPtr, from_raw_mut};
    use serde_derive::Deserialize;

    const RAW_DEVICE_TREE: &[u8] = include_bytes!("../../examples/bl808.dtb");
    const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();
    const RAW_DEVICE_TREE_QEMU: &[u8] = include_bytes!("../../examples/qemu-virt.dtb");
    const BUFFER_SIZE_QEMU: usize = RAW_DEVICE_TREE_QEMU.len();
    #[derive(Deserialize)]
    pub struct Tree<'a> {
        /// Memory information.
        pub memory: NodeSeq<'a>,
    }
    /// Memory range.
    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct Memory<'a> {
        pub reg: Reg<'a>,
    }
    #[test]
    fn test_nodeseq_without_at() {
        #[repr(align(8))]
        struct AlignedBuffer {
            pub data: [u8; RAW_DEVICE_TREE.len()],
        }
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let t: Tree = from_raw_mut(&dtb).unwrap();
        assert_eq!(t.memory.len(), 1);
        let range = t
            .memory
            .iter()
            .next()
            .unwrap()
            .deserialize::<Memory>()
            .reg
            .iter()
            .next()
            .unwrap()
            .0;
        assert_eq!(range, 1342177280..1408237568);
    }
    #[test]
    fn test_nodeseq_with_at() {
        #[repr(align(8))]
        struct AlignedBuffer {
            pub data: [u8; RAW_DEVICE_TREE_QEMU.len()],
        }
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE_QEMU],
        });
        aligned_data.data[..BUFFER_SIZE_QEMU].clone_from_slice(RAW_DEVICE_TREE_QEMU);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let t: Tree = from_raw_mut(&dtb).unwrap();
        assert_eq!(t.memory.len(), 1);
        let range = t
            .memory
            .iter()
            .next()
            .unwrap()
            .deserialize::<Memory>()
            .reg
            .iter()
            .next()
            .unwrap()
            .0;
        assert_eq!(range, 2147483648..6442450944);
    }
}
