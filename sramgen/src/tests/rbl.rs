use crate::config::rbl::*;
use crate::paths::out_bin;
use crate::schematic::rbl::*;
use crate::schematic::{generate_netlist, save_modules};
use crate::tests::test_work_dir;

#[test]
fn test_netlist_replica_bitcell_column() -> Result<(), Box<dyn std::error::Error>> {
    let name = "sramgen_replica_bitcell_column";
    let modules = replica_bitcell_column(ReplicaBitcellColumnParams {
        name: name.to_string(),
        num_active_cells: 8,
        height: 16,
    });

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    Ok(())
}

#[test]
fn test_netlist_replica_column() -> Result<(), Box<dyn std::error::Error>> {
    let name = "sramgen_replica_column";
    let modules = replica_column(ReplicaColumnParams {
        name: name.to_string(),
        bitcell_params: ReplicaBitcellColumnParams {
            name: "replica_bitcell_column".to_string(),
            num_active_cells: 8,
            height: 16,
        },
    });

    let work_dir = test_work_dir(name);

    let bin_path = out_bin(&work_dir, name);
    save_modules(&bin_path, name, modules)?;

    generate_netlist(&bin_path, &work_dir)?;

    Ok(())
}
