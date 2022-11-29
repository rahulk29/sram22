use pdkprims::config::Int;

pub struct DoutBufParams {
    pub name: String,
    pub length: Int,
    pub nw1: Int,
    pub pw1: Int,
    pub nw2: Int,
    pub pw2: Int,
}

pub struct DoutBufArrayParams {
    pub name: String,
    pub width: usize,
    pub mux_ratio: usize,
    pub instance_params: DoutBufParams,
}
