# serde_device_tree

Use [serde](https://serde.rs) framework to deserialize Device Tree Blob binary files; no_std compatible.

## Use this library 

Run example:

```
cargo run --example hifive-unmatched-a00
```

You'll get following results:

```
   Compiling serde_device_tree v0.1.0 (D:\RustSBI\serde_device_tree)
    Finished dev [unoptimized + debuginfo] target(s) in 0.92s
     Running `target\debug\examples\hifive-unmatched-a00.exe`
#address_cells = 2
#size_cells = 2
model = SiFive HiFive Unmatched A00
compatible = sifive,hifive-unmatched-a00sifive,fu740-c000sifive,fu740
stdout = serial0
cpu timebase frequency = 1000000
cpu u_boot_dm_spl = true
cpu cpu@0, compaible = sifive,bullet0riscv
cpu cpu@1, compaible = sifive,bullet0riscv
cpu cpu@2, compaible = sifive,bullet0riscv
cpu cpu@3, compaible = sifive,bullet0riscv
cpu cpu@4, compaible = sifive,bullet0riscv
```
