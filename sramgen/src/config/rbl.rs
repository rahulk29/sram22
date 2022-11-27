#[derive(Debug, Clone)]
pub struct ReplicaBitcellColumnParams {
    pub name: String,
    pub num_active_cells: i64,
    pub height: i64,
}

#[derive(Debug, Clone)]
pub struct ReplicaColumnParams {
    pub name: String,
    pub bitcell_params: ReplicaBitcellColumnParams,
}
