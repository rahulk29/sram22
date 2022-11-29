pub struct TmcUnitParams {
    /// The name of the timing multiplier circuit cell.
    pub name: String,
    /// The timing multiplier (must be at least 2).
    pub multiplier: usize,
}

pub struct TmcParams {
    /// The name of the timing multiplier circuit cell.
    pub name: String,
    /// The timing multiplier (must be at least 2).
    pub multiplier: usize,
    /// The number of delay units.
    pub units: usize,
}
