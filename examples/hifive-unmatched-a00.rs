extern crate alloc;

use serde_derive::Deserialize;
use serde_device_tree::{from_raw_mut, Dtb, DtbPtr, NodeSeq, StrSeq};

static DEVICE_TREE: &[u8] = include_bytes!("hifive-unmatched-a00.dtb");

#[derive(Debug, Deserialize)]
struct Tree<'a> {
    #[serde(rename = "#address-cells")]
    num_address_cells: u32,
    #[serde(rename = "#size-cells")]
    num_size_cells: u32,
    model: &'a str,
    compatible: StrSeq<'a>,
    chosen: Option<Chosen<'a>>,
    cpus: Cpus<'a>,
    memory: NodeSeq<'a, Memory<'a>>,
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
    #[serde(rename = "u-boot,dm-spl", default)]
    u_boot_dm_spl: bool,
    cpu: NodeSeq<'a, Cpu<'a>>,
}

#[derive(Debug, Deserialize)]
struct Cpu<'a> {
    compatible: StrSeq<'a>,
}

#[derive(Debug, Deserialize)]
struct Memory<'a> {
    device_type: &'a str,
}

fn main() {
    let mut slice = DEVICE_TREE.to_vec();
    {
        let ptr = DtbPtr::from_raw(slice.as_mut_ptr()).unwrap();
        let dtb = Dtb::from(ptr).share();
        let t: Tree = from_raw_mut(&dtb).unwrap();
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

        for cpu in t.cpus.cpu.iter() {
            println!(
                "cpu@{}: compatible = {:?}",
                cpu.at(),
                cpu.deserialize().compatible
            );
        }

        for mem in t.memory.iter() {
            println!(
                "memory@{}: device_type = {}",
                mem.at(),
                mem.deserialize().device_type
            );
        }
    }
    assert_eq!(slice, DEVICE_TREE);
}
