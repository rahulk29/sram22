use derive_builder::Builder;
use pdkprims::config::Int;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GridOrder {
    RowMajor,
    ColumnMajor,
}

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
