extern crate alloc;

use serde_derive::Deserialize;
use serde_device_tree::{from_raw_mut, NodeSeq, StrSeq};

static DEVICE_TREE: &[u8] = include_bytes!("hifive-unmatched-a00.dtb");

#[derive(Debug, Deserialize)]
struct Tree {
    #[serde(rename = "#address-cells")]
    num_address_cells: u32,
    #[serde(rename = "#size-cells")]
    num_size_cells: u32,
    model: &'static str,
    compatible: StrSeq,
    #[serde(default)]
    chosen: Option<Chosen>,
    cpus: Cpus,
    memory: NodeSeq<Memory>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Chosen {
    stdout_path: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Cpus {
    timebase_frequency: u32,
    #[serde(rename = "u-boot,dm-spl", default)]
    u_boot_dm_spl: bool,
    cpu: NodeSeq<Cpu>,
}

#[derive(Debug, Deserialize)]
struct Cpu {
    compatible: StrSeq,
}

#[derive(Debug, Deserialize)]
struct Memory {
    device_type: &'static str,
}

fn main() {
    // let ptr = DEVICE_TREE.as_ptr();
    let mut slice = DEVICE_TREE.to_vec();
    let mut t: Tree = unsafe { from_raw_mut(slice.as_mut_ptr()) }.unwrap();
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

    while t.cpus.cpu.exist() {
        println!(
            "cpn@{}, compatible = {:?}",
            t.cpus.cpu.at(),
            t.cpus.cpu.deserialize().unwrap().compatible
        );
        t.cpus.cpu.next();
    }

    println!("memory@{}", t.memory.at());
    println!(
        "memory device_type = {}",
        t.memory.deserialize().unwrap().device_type
    );
}
