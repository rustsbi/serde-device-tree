//! Deserialize device tree data to a Rust data structure,
//! the memory region contains dtb file should be mutable.

use crate::error::Error as DtError;
use serde::de;

mod cursor;
mod data;
// mod group;
mod node;
mod node_seq;
mod reg;
mod str_seq;
// mod r#struct;
mod struct_access;
mod structs;

const VALUE_DESERIALIZER_NAME: &str = "$serde_device_tree$de_mut$ValueDeserializer";

pub use structs::{Dtb, DtbPtr};
pub mod buildin {
    pub use super::{node::Node, node_seq::NodeSeq, reg::Reg, str_seq::StrSeq};
}

use cursor::{BodyCursor, Cursor, PropCursor};
use data::{ValueCursor, ValueDeserializer};
use reg::RegConfig;
use struct_access::{StructAccess, StructAccessType, Temp};
use structs::{RefDtb, StructureBlock, BLOCK_LEN};

/// 从 [`RefDtb`] 反序列化一个描述设备树的 `T` 类型实例。
///
/// 这个函数在没有堆的环境中执行，
/// 因此可以在操作系统启动的极早期或无动态分配的嵌入式系统中使用。
pub fn from_raw_mut<'de, T>(dtb: RefDtb<'de>) -> Result<T, DtError>
where
    T: de::Deserialize<'de>,
{
    // 根节点的名字固定为空字符串，
    // 从一个跳过根节点名字的光标初始化解析器。
    let mut d = ValueDeserializer {
        dtb,
        reg: RegConfig::DEFAULT,
        cursor: ValueCursor::Body(BodyCursor::ROOT),
    };
    T::deserialize(&mut d).and_then(|t| {
        // 解析必须完成
        if d.is_complete_on() {
            Ok(t)
        } else {
            Err(DtError::deserialize_not_complete(d.file_index_on()))
        }
    })
}
