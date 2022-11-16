use crate::generate_netlist;
use crate::schematic::rbl::*;
use crate::utils::save_modules;

#[test]
fn test_netlist_replica_bitcell_column() -> Result<(), Box<dyn std::error::Error>> {
    let name = "sramgen_replica_bitcell_column";
    let modules = replica_bitcell_column(ReplicaBitcellColumnParams {
        name: name.to_string(),
        num_active_cells: 8,
        height: 16,
    });

    save_modules(name, modules)?;

    generate_netlist(name)?;

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

    save_modules(name, modules)?;

    generate_netlist(name)?;

    Ok(())
}
