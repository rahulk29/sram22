pub struct DoutBufParams {
    pub length: Int,
    pub nw1: Int,
    pub pw1: Int,
    pub nw2: Int,
    pub pw2: Int,
}

pub struct DoutBufArrayParams {
    pub name: String,
    pub width: i64,
    pub instance_params: DoutBufParams,
}
