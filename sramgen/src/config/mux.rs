use pdkprims::config::Int;

pub struct ReadMuxParams {
    pub name: String,
    pub length: Int,
    pub width: Int,
}

pub struct ReadMuxArrayParams {
    pub name: String,
    pub mux_params: ReadMuxParams,
    pub cols: usize,
    pub mux_ratio: usize,
}

pub struct WriteMuxParams {
    pub name: String,
    pub length: Int,
    pub width: Int,
    pub wmask: bool,
}

pub struct WriteMuxArrayParams {
    pub name: String,
    pub mux_params: WriteMuxParams,
    pub cols: usize,
    pub mux_ratio: usize,
    pub wmask_width: usize,
}
