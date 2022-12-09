# Schematic API

```rust
let vdd = signal("vdd");
let vss = signal("vss");
let bl = bus("bl", cols);
let br = bus("br", cols);
let wl = bus("wl", rows);
let vnb = signal("vnb");
let vpb = signal("vpb");
let rbl = signal("rbl");
let rbr = signal("rbr");

let mut module = Module::new(&params.name);
module.expose_ports(&[&vdd, &vss, &bl, &br, &vnb, &vpb], Direction::Inout);
module.expose_ports(&[&wl], Direction::Input);

for i in 0..total_rows {
  for j in 0..total_cols {
    let mut inst = Instance::new(&format!("bitcell_{}_{}", i, j), sram_sp_cell_ref());
    inst.connect_ports(vec![
      ("VDD", &vdd),
      ("VSS", &vss),
      ("VNB", &vnb),
      ("VPB", &vpb),
      ("WL", &wl[i]),
      ("BL", &bl[j]),
    ]);
    module.instances.push(inst);
  }
}
```
