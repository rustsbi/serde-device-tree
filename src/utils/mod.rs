use crate::buildin::Node;

pub trait DeviceTreeTraversal {
    fn find(&self, _path: &str) -> Option<Node> {
        None
    }
    fn search<F>(&self, _func: &mut F)
    where
        F: FnMut(&Node),
    {
    }
}

impl DeviceTreeTraversal for Node<'_> {
    /// Try to get a node by path
    fn find(&self, path: &str) -> Option<Node> {
        // Direct return root node
        let mut current_node = Some(self.clone());
        if path == "/" {
            return current_node;
        }
        let (root, path) = path.split_at(1);
        if root != "/" {
            return None;
        }
        // Split path with / and find each level
        for current_name in path.split('/') {
            let node = match current_node.clone() {
                Some(node) => node,
                None => break,
            };
            let mut nodes = match node.nodes() {
                Some(nodes) => nodes,
                None => {
                    current_node = None;
                    break;
                }
            };
            let next_node_iter = nodes.find(|x| x.get_full_name() == current_name);
            match next_node_iter {
                None => current_node = None,
                Some(iter) => {
                    let next_node = iter.deserialize::<Node>();
                    current_node = Some(next_node);
                }
            }
        }
        current_node
    }

    /// use depth-first search to traversal the tree, and exec func for each node
    fn search<F>(&self, func: &mut F)
    where
        F: FnMut(&Node),
    {
        func(self);
        if let Some(nodes) = self.nodes() {
            for node in nodes {
                let node = node.deserialize::<Node>();
                node.search(func);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        buildin::{Node, StrSeq},
        from_raw_mut, Dtb, DtbPtr,
    };
    static RAW_DEVICE_TREE: &'static [u8] =
        include_bytes!("../../examples/hifive-unmatched-a00.dtb");
    const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();
    #[repr(align(8))]
    struct AlignedBuffer {
        pub data: [u8; RAW_DEVICE_TREE.len()],
    }
    #[test]
    fn test_search() {
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Node = from_raw_mut(&dtb).unwrap();
        let mut count = 0;
        let mut closure = |_node: &Node| count += 1;
        node.search(&mut closure);
        assert_eq!(count, 70);
    }
    #[test]
    fn test_find() {
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE],
        });
        aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Node = from_raw_mut(&dtb).unwrap();
        let node = node.find("/chosen").unwrap();
        let result = node
            .props()
            .unwrap()
            .find(|prop| prop.get_name() == "stdout-path");
        match result {
            Some(iter) => {
                if iter.deserialize::<StrSeq>().iter().next().unwrap() != "serial0" {
                    panic!("wrong /chosen/stdout-path value");
                }
            }
            None => panic!("failed to find /chosen/stdout-path"),
        }
    }
}
