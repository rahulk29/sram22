#[derive(Debug, Clone)]
pub struct PrechargeParams {
    pub name: String,
    pub length: Int,
    pub pull_up_width: Int,
    pub equalizer_width: Int,
}

#[derive(Debug, Clone)]
pub struct PrechargeArrayParams {
    pub name: String,
    pub width: usize,
    pub instance_params: PrechargeParams,
}
