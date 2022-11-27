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
