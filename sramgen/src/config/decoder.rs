use layout21::raw::geom::Dir;
use pdkprims::config::Int;
use serde::{Deserialize, Serialize};

use crate::config::gate::{GateParams, Size};
use crate::schematic::decoder::DecoderTree;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct DecoderParams {
    pub name: String,
    pub tree: DecoderTree,
    pub lch: Int,
}

pub struct Decoder24Params {
    pub name: String,
    pub gate_size: Size,
    pub inv_size: Size,
    pub lch: Int,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GateDecArrayParams {
    pub name: String,
    pub width: usize,
    pub dir: Dir,
    pub pitch: Option<Int>,
}

pub struct NandDecArrayParams {
    pub array_params: GateDecArrayParams,
    pub gate: GateParams,
    pub gate_size: usize,
}

pub struct AndDecArrayParams {
    pub array_params: GateDecArrayParams,
    pub nand: GateParams,
    pub inv: GateParams,
    pub gate_size: usize,
}

pub fn nand2_dec_params(name: impl Into<String>) -> GateParams {
    GateParams {
        name: name.into(),
        size: Size {
            nmos_width: 3_200,
            pmos_width: 2_400,
        },
        length: 150,
    }
}
