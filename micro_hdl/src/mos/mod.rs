use crate::primitive::mos::{Flavor, Intent};

pub fn sky130_mos_name(f: Flavor, _i: Intent) -> String {
    (match f {
        Flavor::Nmos => "sky130_fd_pr__nfet_01v8",
        Flavor::Pmos => "sky130_fd_pr__pfet_01v8",
    })
    .into()
}
