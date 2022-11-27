#[derive(Clone, Eq, PartialEq, Builder)]
pub struct DffGridParams {
    #[builder(setter(into))]
    pub name: String,
    pub rows: usize,
    pub cols: usize,
    #[builder(setter(strip_option), default)]
    pub row_pitch: Option<Int>,
    #[builder(default = "GridOrder::ColumnMajor")]
    pub order: GridOrder,
}

impl DffGridParams {
    #[inline]
    pub fn builder() -> DffGridParamsBuilder {
        DffGridParamsBuilder::default()
    }
}
