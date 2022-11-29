pub struct InvChainParams {
    pub name: String,
    pub num: usize,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct InvChainGridParams {
    pub name: String,
    pub rows: usize,
    pub cols: usize,
}
