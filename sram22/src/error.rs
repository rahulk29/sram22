use thiserror::Error;

#[derive(Debug, Error)]
pub enum Sram22Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Sram22Error>;
