use crate::config::rbl::*;
use crate::paths::out_bin;
use crate::schematic::rbl::*;
use crate::schematic::{generate_netlist, save_modules};
use crate::tests::test_work_dir;

#[test]
fn test_netlist_replica_bitcell_column() -> Result<(), Box<dyn std::error::Error>> {
    let name = "sramgen_replica_bitcell_column";
    let modules = replica_bitcell_column(&ReplicaBitcellColumnParams {
        name: name.to_string(),
        rows: 64,
        dummy_rows: 0,
    });

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    Ok(())
}

#[test]
fn test_netlist_replica_bitcell_column_dummies() -> Result<(), Box<dyn std::error::Error>> {
    let name = "sramgen_replica_column_dummies";
    let modules = replica_bitcell_column(&ReplicaBitcellColumnParams {
        name: name.to_string(),
        rows: 32,
        dummy_rows: 2,
    });

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    Ok(())
}
