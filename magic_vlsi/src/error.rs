use thiserror::Error as ThisError;

pub type BoxedError = std::boxed::Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, MagicError>;

#[derive(Debug, ThisError)]
pub enum MagicError {
    #[error("I/O error while communicating with magic process: {0}")]
    Io(#[from] std::io::Error),

    #[error("could not decode data from magic process: {0}")]
    Encoding(#[from] std::str::Utf8Error),

    #[error("magic produced unexpected output: {0}")]
    UnexpectedOutput(String),
}

#[derive(Debug, ThisError)]
pub enum StartMagicError {
    #[error("failed to start magic process: {0}")]
    Spawn(#[source] BoxedError),

    #[error("failed to connect to magic process: {0}")]
    Connect(String),

    #[error("I/O error while communicating with magic process: {0}")]
    Io(#[from] std::io::Error),

    #[error("error while initializing magic: {0}")]
    Init(#[from] MagicError),
}
