use crate::config::gate::AndParams;

#[derive(Debug, Clone)]
pub struct WriteMaskControlParams {
    pub name: String,
    pub width: i64,
    pub and_params: AndParams,
}
