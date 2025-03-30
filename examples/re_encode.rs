use serde_device_tree::ser::{patch::Patch, serializer::ValueType};
use serde_device_tree::{Dtb, DtbPtr, buildin::Node, error::Error, from_raw_mut};

use std::io::prelude::*;

const RAW_DEVICE_TREE: &[u8] = include_bytes!("qemu-virt.dtb");
const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();

#[repr(align(8))]
struct AlignedBuffer {
    pub data: [u8; RAW_DEVICE_TREE.len()],
}

fn main() -> Result<(), Error> {
    let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
        data: [0; BUFFER_SIZE],
    });
    aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
    let mut buf = [0u8; RAW_DEVICE_TREE.len() * 2];
    let mut slice = aligned_data.data.to_vec();
    let ptr = DtbPtr::from_raw(slice.as_mut_ptr())?;
    let dtb = Dtb::from(ptr).share();

    let root: Node = from_raw_mut(&dtb).unwrap();
    let patch: Patch = Patch::new("/chosen/a", &"1", ValueType::Prop);
    serde_device_tree::ser::to_dtb(&root, &[patch], &mut buf).unwrap();

    let mut file = std::fs::File::create("gen.dtb").unwrap();
    file.write_all(&buf).unwrap();

    Ok(())
}
