//! 这是一个 `from_raw_mut` 反序列化设备树的示例。不需要 `alloc`。
// extern crate alloc;

// 在实际使用中，将这里的 `serde_derive::Deserialize` 改为 `serde::Deserialize`。
use serde_derive::Deserialize;

// - `DtbPtr`: 验证设备树首部正确性，后续也可借助这个类型传递设备树，多次解析不必重复验证。
// - `Dtb`: 管理反序列化出的类型生命周期。
// - `from_raw_mut`: 反序列化。
// - `Reg`: 常见属性。其值解析方式由 `#address-cells` 和 `#size-cells` 决定。
// - `NodeSeq`: name@... 区分的一组同级同类的连续节点，这个类型要求可变的内存。
// - `StrSeq`: '\0' 分隔的一组字符串，设备树中一种常见的属性类型，这个类型要求可变的内存。
use serde_device_tree::{
    buildin::{Node, NodeSeq, Reg, StrSeq},
    error::Error,
    from_raw_mut, Dtb, DtbPtr,
};

const RAW_DEVICE_TREE: &[u8] = include_bytes!("qemu-virt.dtb");
const BUFFER_SIZE: usize = RAW_DEVICE_TREE.len();

#[repr(align(8))]
struct AlignedBuffer {
    pub data: [u8; RAW_DEVICE_TREE.len()],
}

fn main() -> Result<(), Error> {
    // 整个设备树二进制文件需要装载到一块可写的内存区域
    let mut aligned_data: Box<AlignedBuffer> = Box::new(AlignedBuffer {
        data: [0; BUFFER_SIZE],
    });
    aligned_data.data[..BUFFER_SIZE].clone_from_slice(RAW_DEVICE_TREE);
    let mut slice = aligned_data.data.to_vec();
    // 这一步验证了设备树首部的正确性，`DtbPtr` 类型可以安全地传递到任何地方，
    // 甚至跨地址空间（如果你知道偏移的话）。
    let ptr = DtbPtr::from_raw(slice.as_mut_ptr())?;
    // 构造一个方便解析的 Dtb 结构，这个结构不再支持跨线程传递
    let dtb = Dtb::from(ptr).share();

    // 实际使用中，将类型定义在专门的位置更合适，
    // 这里是为了阅读的顺序考虑。
    //
    // 关于 `#[derive(Deserialize)]`，看[这篇文档](https://serde.rs/derive.html)。
    // 关于 `rename` 等 Attribute，看[这篇文档](https://serde.rs/attributes.html)。
    //
    // 推荐用 `StrSeq<'a>` 替换所有 `&'a str`，即使肯定只有一个字符串。
    // 后者在 `derive` 时可能引发奇怪的生命周期问题。
    //
    // 许多外设可能有不止一个，用 @... 区分，用 `NodeSeq` 映射这类节点。
    // 注意！解析器要求这类节点必须连续出现。

    #[derive(Deserialize)]
    struct Tree<'a> {
        compatible: StrSeq<'a>,
        model: StrSeq<'a>,
        chosen: Option<Chosen<'a>>,
        cpus: Cpus<'a>,
        memory: NodeSeq<'a>,
        soc: Node<'a>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    struct Chosen<'a> {
        stdout_path: Option<StrSeq<'a>>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    struct Cpus<'a> {
        timebase_frequency: u32,
        cpu: NodeSeq<'a>,
    }

    #[allow(dead_code)]
    #[derive(Deserialize, Debug)]
    struct Cpu<'a> {
        compatible: StrSeq<'a>,
        device_type: StrSeq<'a>,
        status: StrSeq<'a>,
        #[serde(rename = "riscv,isa")]
        isa: StrSeq<'a>,
        #[serde(rename = "mmu-type")]
        mmu: StrSeq<'a>,
    }

    #[derive(Deserialize)]
    struct Memory<'a> {
        device_type: StrSeq<'a>,
        reg: Reg<'a>,
    }

    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct Soc<'a> {
        virtio_mmio: NodeSeq<'a>,
    }

    #[derive(Deserialize)]
    struct VirtIoMmio<'a> {
        reg: Reg<'a>,
    }

    {
        // 解析！
        let t: Tree = from_raw_mut(&dtb).unwrap();

        println!("model = {:?}", t.model);
        println!("compatible = {:?}", t.compatible);
        if let Some(chosen) = t.chosen {
            if let Some(stdout_path) = chosen.stdout_path {
                println!("stdout = {:?}", stdout_path);
            } else {
                println!("stdout not chosen");
            }
        }
        println!("cpu timebase frequency = {}", t.cpus.timebase_frequency);

        // 可以读取同类节点的数量
        println!("number of cpu = {}", t.cpus.cpu.len());
        for cpu in t.cpus.cpu.iter() {
            println!("cpu@{}: {:?}", cpu.at(), cpu.deserialize::<Cpu>());
        }

        for item in t.memory.iter() {
            let mem: Memory = item.deserialize();
            println!(
                "memory@{}({:?}): {:#x?}",
                item.at(),
                mem.device_type,
                mem.reg
            );
        }

        println!("{:?}", t.soc);
        for current_node in t.soc.nodes() {
            if current_node.get_parsed_name().0 == "virtio_mmio" {
                let mmio = current_node.deserialize::<VirtIoMmio>();
                println!("{:?} {:?}", current_node.get_parsed_name(), mmio.reg);
            }
        }

        // 解析过程中，设备树的内容被修改了。
        // 因此若要以其他方式再次访问设备树，先将这次解析的结果释放。
        //        assert_ne!(slice, RAW_DEVICE_TREE);
    }
    // 释放后，内存会恢复原状。
    assert_eq!(slice, RAW_DEVICE_TREE);

    Ok(())
}
