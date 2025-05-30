pub(crate) const SKY130_DRC_RULES_PATH: &str = concat!(
    env!("SKY130_COMMERCIAL_PDK_ROOT"),
    "/PV/Calibre/DRC/calibre_drc.rul"
);
pub(crate) const SKY130_LAYERPROPS_PATH: &str = "/tools/C/ethanwu10/sky130/nda/sky130.layerprops";
pub(crate) const SKY130_LVS_RULES_PATH: &str = concat!(
    env!("SKY130_COMMERCIAL_PDK_ROOT"),
    "/PV/Calibre/LVS/calibre_lvs.rul"
);
pub(crate) const SKY130_PEX_RULES_PATH: &str = concat!(
    env!("SKY130_COMMERCIAL_PDK_ROOT"),
    "/PV/Calibre/PEX/calibre_pex.rul"
);
