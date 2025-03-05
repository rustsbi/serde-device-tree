use serde::Deserialize;

use serde_device_tree::{Dtb, DtbPtr, buildin::NodeSeq, error::Error, from_raw_mut};

const RAW_DEVICE_TREE: &[u8] = include_bytes!("../examples/bl808.dtb");
const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();

#[repr(align(8))]
struct AlignedBuffer {
    pub data: [u8; RAW_DEVICE_TREE.len()],
}

/// Root device tree structure containing system information.
#[derive(Deserialize)]
pub struct Tree<'a> {
    /// Memory information.
    pub memory: NodeSeq<'a>,
}

#[test]
fn bl808() -> Result<(), Error> {
    // 整个设备树二进制文件需要装载到一块可写的内存区域
    let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
        data: [0; BUFFER_SIZE],
    });
    aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
    let mut slice = aligned_data.data.to_vec();
    let ptr = DtbPtr::from_raw(slice.as_mut_ptr())?;
    let dtb = Dtb::from(ptr).share();

    let _: Tree = from_raw_mut(&dtb).unwrap();

    Ok(())
}
