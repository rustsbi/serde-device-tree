extern crate alloc;

use alloc::collections::BTreeMap;
use serde_derive::Deserialize;
use serde_device_tree::Compatible;

#[derive(Debug, Deserialize)]
struct Tree<'a> {
    #[serde(rename = "#address-cells")]
    num_address_cells: u32,
    #[serde(rename = "#size-cells")]
    num_size_cells: u32,
    model: &'a str,
    compatible: Compatible<'a>,
    chosen: Option<Chosen<'a>>,
    cpus: Cpus<'a>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Chosen<'a> {
    stdout_path: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Cpus<'a> {
    timebase_frequency: u32,
    #[serde(rename = "u-boot,dm-spl")]
    u_boot_dm_spl: bool,
    #[serde(flatten, borrow)]
    cpu: BTreeMap<&'a str, MaybeCpu<'a>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MaybeCpu<'a> {
    Cpu(Cpu<'a>),
    #[allow(unused)]
    Bytes(&'a [u8]),
}

#[derive(Debug, Deserialize)]
struct Cpu<'a> {
    #[serde(borrow)]
    compatible: Compatible<'a>,
}

const RAW_DEVICE_TREE: &'static [u8] = include_bytes!("hifive-unmatched-a00.dtb");
const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();

#[repr(align(4))]
struct AlignedBuffer {
    pub data: [u8; RAW_DEVICE_TREE.len()],
}

fn main() {
    let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
        data: [0; BUFFER_SIZE],
    });
    aligned_data.data[..BUFFER_SIZE].clone_from_slice(&RAW_DEVICE_TREE);
    let ptr = aligned_data.data.as_ptr();
    let t: Tree = unsafe { serde_device_tree::from_raw(ptr) }.unwrap();
    println!("#address_cells = {}", t.num_address_cells);
    println!("#size_cells = {}", t.num_size_cells);
    println!("model = {}", t.model);
    println!("compatible = {:?}", t.compatible);
    if let Some(chosen) = t.chosen {
        if let Some(stdout_path) = chosen.stdout_path {
            println!("stdout = {}", stdout_path);
        } else {
            println!("stdout not chosen");
        }
    }
    println!("cpu timebase frequency = {}", t.cpus.timebase_frequency);
    println!("cpu u_boot_dm_spl = {}", t.cpus.u_boot_dm_spl);
    for (cpu_name, cpu) in t.cpus.cpu {
        if let MaybeCpu::Cpu(cpu) = cpu {
            println!("cpu {}, compaible = {:?}", cpu_name, cpu.compatible)
        }
    }
}
