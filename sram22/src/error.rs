use thiserror::Error;

#[derive(Debug, Error)]
pub enum Sram22Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("magic error: {0}")]
    Magic(#[from] magic_vlsi::error::MagicError),
}

pub type Result<T> = std::result::Result<T, Sram22Error>;
