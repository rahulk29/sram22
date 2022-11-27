#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BitcellArrayParams {
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    pub dummy_rows: usize,
    pub dummy_cols: usize,
}
