use serde_derive::Deserialize;
use serde_device_tree::Compatible;

#[derive(Debug, Deserialize)]
struct Tree<'a> {
    #[serde(rename = "#address-cells")]
    num_address_cells: u32,
    #[serde(rename = "#size-cells")]
    num_size_cells: u32,
    model: &'a str,
    #[allow(unused)]
    compatible: Compatible<'a>,
    chosen: Option<Chosen<'a>>,
    cpus: Cpus,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Chosen<'a> {
    stdout_path: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Cpus {
    timebase_frequency: u32,
    #[serde(rename = "u-boot,dm-spl")]
    u_boot_dm_spl: bool,
}

const RAW_DEVICE_TREE: &[u8] = include_bytes!("../examples/hifive-unmatched-a00.dtb");
const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();

#[repr(align(4))]
struct AlignedBuffer {
    pub data: [u8; RAW_DEVICE_TREE.len()],
}

#[test]
fn hifive_unmatched() {
    let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
        data: [0; BUFFER_SIZE],
    });
    aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
    let ptr = aligned_data.data.as_ptr();
    let t: Tree = unsafe { serde_device_tree::from_raw(ptr) }.unwrap();
    assert_eq!(t.num_address_cells, 2);
    assert_eq!(t.num_size_cells, 2);
    assert_eq!(t.model, "SiFive HiFive Unmatched A00\0");
    if let Some(chosen) = t.chosen {
        if let Some(stdout_path) = chosen.stdout_path {
            assert_eq!(stdout_path, "serial0\0");
        } else {
            assert!(false, "Failed to find chosen/stdout_path");
        }
    }
    assert_eq!(t.cpus.timebase_frequency, 1000000);
    assert_eq!(t.cpus.u_boot_dm_spl, true);
}
