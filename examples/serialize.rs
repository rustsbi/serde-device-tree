use serde::Serialize;
use std::io::prelude::*;

use serde_device_tree::ser::serializer::ValueType;

const MAX_SIZE: usize = 1024;

fn main() {
    #[derive(Serialize)]
    struct Base {
        pub hello: u32,
        pub base1: Base1,
        pub hello2: u32,
        pub base2: Base1,
    }
    #[derive(Serialize)]
    struct Base1 {
        pub hello: &'static str,
    }
    #[derive(Serialize)]
    struct ReversedMemory {
        #[serde(rename = "#address-cells")]
        pub address_cell: u32,
        #[serde(rename = "#size-cells")]
        pub size_cell: u32,
        pub ranges: (),
    }
    #[derive(Serialize)]
    struct ReversedMemoryItem {
        pub reg: [u32; 4],
    }
    let mut buf1 = [0u8; MAX_SIZE];

    {
        let new_base = ReversedMemory {
            address_cell: 2,
            size_cell: 2,
            ranges: (),
        };
        let new_base_2 = ReversedMemoryItem { reg: [0, 1, 0, 20] };
        let patch1 = serde_device_tree::ser::patch::Patch::new(
            "/reversed-memory",
            &new_base as _,
            ValueType::Node,
        );
        let patch2 = serde_device_tree::ser::patch::Patch::new(
            "/reversed-memory/mmode_resv1@0",
            &new_base_2 as _,
            ValueType::Node,
        );
        let list = [patch1, patch2];
        let base = Base {
            hello: 0xdeedbeef,
            base1: Base1 {
                hello: "Hello, World!",
            },
            hello2: 0x11223344,
            base2: Base1 { hello: "Roger" },
        };
        serde_device_tree::ser::to_dtb(&base, &list, &mut buf1).unwrap();
    }
    let mut file = std::fs::File::create("gen.dtb").unwrap();
    file.write_all(&buf1).unwrap();
}
