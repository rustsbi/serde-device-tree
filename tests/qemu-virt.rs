// 在实际使用中，将这里的 `serde_derive::Deserialize` 改为 `serde::Deserialize`。
use serde_derive::Deserialize;

use serde_device_tree::{
    Dtb, DtbPtr,
    buildin::{NodeSeq, Reg},
    error::Error,
    from_raw_mut,
};

const RAW_DEVICE_TREE: &[u8] = include_bytes!("../examples/qemu-virt.dtb");
const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();

#[repr(align(8))]
struct AlignedBuffer {
    pub data: [u8; RAW_DEVICE_TREE.len()],
}

#[derive(Deserialize)]
struct Tree<'a> {
    soc: Soc<'a>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Soc<'a> {
    virtio_mmio: NodeSeq<'a>,
}

#[derive(Deserialize)]
#[allow(unused)]
struct VirtIoMmio<'a> {
    reg: Reg<'a>,
}

#[test]
fn qemu_virt() -> Result<(), Error> {
    // 整个设备树二进制文件需要装载到一块可写的内存区域
    let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
        data: [0; BUFFER_SIZE],
    });
    aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
    let mut slice = aligned_data.data.to_vec();
    let ptr = DtbPtr::from_raw(slice.as_mut_ptr())?;
    let dtb = Dtb::from(ptr).share();

    let t: Tree = from_raw_mut(&dtb).unwrap();

    assert_eq!(t.soc.virtio_mmio.len(), 8);
    assert_eq!(slice, RAW_DEVICE_TREE);

    Ok(())
}
