extern crate alloc;

use serde_derive::Deserialize;
use serde_device_tree::{from_raw_mut, Dtb, DtbPtr, NodeSeq, StrSeq};

static DEVICE_TREE: &[u8] = include_bytes!("qemu-virt.dtb");

#[derive(Debug, Deserialize)]
struct Tree<'a> {
    compatible: StrSeq<'a>,
    chosen: Option<Chosen<'a>>,
    cpus: Cpus<'a>,
    memory: NodeSeq<'a, Memory<'a>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Chosen<'a> {
    stdout_path: Option<StrSeq<'a>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Cpus<'a> {
    timebase_frequency: u32,
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
        println!("compatible = {:?}", t.compatible);
        if let Some(chosen) = t.chosen {
            if let Some(stdout_path) = chosen.stdout_path {
                println!("stdout = {:?}", stdout_path);
            } else {
                println!("stdout not chosen");
            }
        }
        println!("cpu timebase frequency = {}", t.cpus.timebase_frequency);

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
