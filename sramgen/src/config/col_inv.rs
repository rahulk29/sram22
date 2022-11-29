use pdkprims::config::Int;

pub struct ColInvParams {
    pub name: String,
    pub length: Int,
    pub nwidth: Int,
    pub pwidth: Int,
}

pub struct ColInvArrayParams {
    pub name: String,
    pub width: usize,
    pub mux_ratio: usize,
    pub instance_params: ColInvParams,
}
