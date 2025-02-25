use crate::buildin::{Node, StrSeq};

impl<'de> Node<'de> {
    /// Get node /chosen
    #[inline]
    pub fn chosen<'b>(&'b self) -> Option<Node<'de>> {
        self.find("/chosen")
    }
    /// Get /chosen/stdin-path
    pub fn chosen_stdin_path(&self) -> Option<&'de str> {
        let result = self
            .chosen()?
            .get_prop("stdin-path")?
            .deserialize::<StrSeq>()
            .iter()
            .next()?;
        if let Some(pos) = result.find(':') {
            Some(result.split_at(pos).0)
        } else {
            Some(result)
        }
    }
    /// Get /chosen/stdout-path
    pub fn chosen_stdout_path(&self) -> Option<&'de str> {
        let result = self
            .chosen()?
            .get_prop("stdout-path")?
            .deserialize::<StrSeq>()
            .iter()
            .next()?;
        if let Some(pos) = result.find(':') {
            Some(result.split_at(pos).0)
        } else {
            Some(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Dtb, DtbPtr, buildin::Node, from_raw_mut};

    const RAW_DEVICE_TREE: &[u8] = include_bytes!("../../examples/bl808.dtb");
    const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();
    #[test]
    fn test_chosen_stdout() {
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

        let node: Node = from_raw_mut(&dtb).unwrap();
        assert!(node.chosen().is_some());
        assert_eq!(node.chosen_stdout_path(), Some("serial3"));
    }
}
