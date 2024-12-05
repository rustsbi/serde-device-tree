use crate::buildin::{Node, StrSeq};

impl Node<'_> {
    /// Try to get a node by a full-path.
    fn raw_find(&self, path: &str) -> Option<Node> {
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
            let next_node_iter = node.nodes().find(|x| x.get_full_name() == current_name);
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
    /// Try to get a node by path.
    pub fn find(&self, path: &str) -> Option<Node> {
        // Direct return root node
        let current_node = Some(self.clone());
        if path == "/" {
            return current_node;
        }
        let (root, _) = path.split_at(1);
        if root != "/" {
            // Path name does not start with `/`, Check if the aliases.
            if let Some(aliases) = self.raw_find("/aliases") {
                if let Some(full_path) = aliases.get_prop(path) {
                    // As spec 3.3 said, this prop value should be one string,
                    // which is a full path ref to a node.
                    let full_path = full_path.deserialize::<StrSeq>();
                    return self.raw_find(full_path.iter().next().unwrap());
                }
            }
            return None;
        }
        return self.raw_find(path);
    }

    /// use depth-first search to traversal the tree, and exec func for each node
    pub fn search<F>(&self, func: &mut F)
    where
        F: FnMut(&Node),
    {
        func(self);
        for node in self.nodes() {
            let node = node.deserialize::<Node>();
            node.search(func);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        buildin::{Node, StrSeq},
        from_raw_mut, Dtb, DtbPtr,
    };
    const RAW_DEVICE_TREE: &[u8] = include_bytes!("../../examples/hifive-unmatched-a00.dtb");
    const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();

    const RAW_DEVICE_TREE_WITH_ALIASES: &[u8] =
        include_bytes!("../../examples/cv1812cp_milkv_duo256m_sd.dtb");
    const BUFFER_SIZE_WITH_ALIASES: usize = RAW_DEVICE_TREE_WITH_ALIASES.len();
    #[test]
    fn test_search() {
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
        let mut count = 0;
        let mut closure = |_node: &Node| count += 1;
        node.search(&mut closure);
        assert_eq!(count, 70);
    }
    #[test]
    fn test_find() {
        #[repr(align(8))]
        struct AlignedBuffer {
            pub data: [u8; RAW_DEVICE_TREE_WITH_ALIASES.len()],
        }
        let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
            data: [0; BUFFER_SIZE_WITH_ALIASES],
        });
        aligned_data.data[..BUFFER_SIZE_WITH_ALIASES]
            .clone_from_slice(RAW_DEVICE_TREE_WITH_ALIASES);
        let mut slice = aligned_data.data.to_vec();
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();

        let node: Node = from_raw_mut(&dtb).unwrap();
        let chosen = node.find("/chosen").unwrap();
        let result = chosen.props().find(|prop| prop.get_name() == "stdout-path");
        match result {
            Some(iter) => {
                let stdout_path = String::from(iter.deserialize::<StrSeq>().iter().next().unwrap());
                if stdout_path != "serial0" {
                    panic!("wrong /chosen/stdout-path value");
                }
                if let None = node.find(&stdout_path) {
                    panic!("unable to find stdout-path node.");
                }
            }
            None => panic!("failed to find /chosen/stdout-path"),
        }
    }
}
