pub struct InvChainParams<'a> {
    pub prefix: &'a str,
    pub num: usize,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct InvChainGridParams<'a> {
    pub prefix: &'a str,
    pub rows: usize,
    pub cols: usize,
}
