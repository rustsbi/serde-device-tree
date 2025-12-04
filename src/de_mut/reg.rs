use super::{BLOCK_LEN, PropCursor, RefDtb, ValueCursor};
use core::{fmt::Debug, ops::Range};
use serde::{Deserialize, Serialize};

/// 节点地址空间。
pub struct Reg<'de>(Inner<'de>);

pub(super) struct Inner<'de> {
    pub dtb: RefDtb<'de>,
    pub cursor: PropCursor,
    pub reg: RegConfig,
}

/// 地址段迭代器。
pub struct RegIter<'de> {
    data: &'de [u8],
    config: RegConfig,
}

#[derive(Clone, Debug)]
pub struct RegRegion(pub Range<usize>);

/// 节点地址空间格式。
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub(super) struct RegConfig {
    pub address_cells: usize,
    pub size_cells: usize,
}

impl RegConfig {
    pub const DEFAULT: Self = Self {
        address_cells: 2,
        size_cells: 1,
    };
}

impl<'de> Deserialize<'de> for Reg<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value_deserialzer = super::ValueDeserializer::deserialize(deserializer)?;

        let inner = Inner {
            dtb: value_deserialzer.dtb,
            reg: value_deserialzer.self_reg,
            cursor: match value_deserialzer.cursor {
                ValueCursor::Prop(_, cursor) => cursor,
                _ => {
                    unreachable!("Reg Deserialize should only be called by prop cursor")
                }
            },
        };

        Ok(Self(inner))
    }
}

impl Reg<'_> {
    pub fn iter(&self) -> RegIter<'_> {
        RegIter {
            data: self.0.cursor.data_on(self.0.dtb),
            config: self.0.reg,
        }
    }
}

impl Debug for Reg<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut iter = self.iter();
        if let Some(s) = iter.next() {
            write!(f, "[{:#x?}", s.0)?;
            for s in iter {
                write!(f, ", {:#x?}", s.0)?;
            }
            write!(f, "]")
        } else {
            write!(f, "[]")
        }
    }
}

impl Iterator for RegIter<'_> {
    type Item = RegRegion;

    fn next(&mut self) -> Option<Self::Item> {
        let len = BLOCK_LEN * (self.config.address_cells + self.config.size_cells);
        if self.data.len() >= len {
            let (current_block, data) = self.data.split_at(len);
            self.data = data;
            let mut base = 0;
            let mut len = 0;
            let mut block_id = 0;
            for _ in 0..self.config.address_cells {
                base = (base << 32)
                    | u32::from_be_bytes(
                        current_block[block_id * 4..(block_id + 1) * 4]
                            .try_into()
                            .unwrap(),
                    ) as usize;
                block_id += 1;
            }
            for _ in 0..self.config.size_cells {
                len = (len << 32)
                    | u32::from_be_bytes(
                        current_block[block_id * 4..(block_id + 1) * 4]
                            .try_into()
                            .unwrap(),
                    ) as usize;
                block_id += 1;
            }
            Some(RegRegion(base..base + len))
        } else {
            None
        }
    }
}

impl Serialize for Reg<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Pass bytes directly for Reg.
        serializer.serialize_bytes(self.0.cursor.data_on(self.0.dtb))
    }
}

#[cfg(test)]
mod tests {
    use crate::buildin::{Node, NodeSeq, Reg};
    use crate::{Dtb, DtbPtr, from_raw_mut};
    use serde::Deserialize;

    const RAW_DEVICE_TREE: &[u8] = include_bytes!("../../examples/reg-test.dtb");
    const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();
    #[repr(align(8))]
    struct AlignedBuffer {
        pub data: [u8; RAW_DEVICE_TREE.len()],
    }

    /// Memory range.
    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub struct Memory<'a> {
        pub reg: Reg<'a>,
    }
    #[test]
    fn test_normal_reg() {
        #[derive(Deserialize)]
        pub struct Tree<'a> {
            /// Memory information.
            pub normal: Memory<'a>,
        }
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Tree = from_raw_mut(&dtb).unwrap();
        assert_eq!(
            node.normal.reg.iter().next().unwrap().0,
            1342177280..1408237568
        );
    }
    #[test]
    fn test_normal_reg_node() {
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Node = from_raw_mut(&dtb).unwrap();
        let reg = node
            .find("/normal")
            .unwrap()
            .get_prop("reg")
            .unwrap()
            .deserialize::<Reg>();
        assert_eq!(reg.iter().next().unwrap().0, 1342177280..1408237568);
    }
    #[test]
    fn test_depper_normal_reg_node() {
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Node = from_raw_mut(&dtb).unwrap();
        let reg = node
            .find("/seq/node@2/normal")
            .unwrap()
            .get_prop("reg")
            .unwrap()
            .deserialize::<Reg>();
        assert_eq!(reg.iter().next().unwrap().0, 1342177280..1408237568);
    }
    #[test]
    fn test_seq_reg_node() {
        #[derive(Deserialize)]
        pub struct Tree<'a> {
            pub seq: Seq<'a>,
        }
        #[derive(Deserialize)]
        pub struct Seq<'a> {
            pub node: NodeSeq<'a>,
        }
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Tree = from_raw_mut(&dtb).unwrap();
        let mut iter = node.seq.node.iter();
        let node1 = iter.next().unwrap();
        assert_eq!(
            node1.deserialize::<Memory>().reg.iter().next().unwrap().0,
            1..4294967298
        );
        let node2 = iter.next().unwrap();
        assert_eq!(
            node2.deserialize::<Memory>().reg.iter().next().unwrap().0,
            2..4
        );
        let node3 = iter.next().unwrap();
        assert_eq!(
            node3
                .deserialize::<Node>()
                .get_prop("reg")
                .unwrap()
                .deserialize::<Reg>()
                .iter()
                .next()
                .unwrap()
                .0,
            3..12884901894
        );
    }
}
