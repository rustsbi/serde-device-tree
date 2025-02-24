use serde_derive::Serialize;
use std::io::prelude::*;

const MAX_SIZE: usize = 256 + 32;

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
    let mut buf1 = [0u8; MAX_SIZE];

    {
        let new_base = Base1 { hello: "added" };
        let patch = serde_device_tree::ser::patch::Patch::new("/base3", &new_base as _);
        let list = [patch];
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
