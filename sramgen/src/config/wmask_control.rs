use crate::config::gate::AndParams;

#[derive(Debug, Clone)]
pub struct WriteMaskControlParams {
    pub name: String,
    pub width: usize,
    pub and_params: AndParams,
}
