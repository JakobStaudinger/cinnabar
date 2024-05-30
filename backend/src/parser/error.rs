use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParserError {
    #[error("{0}")]
    File(String),
    #[error("{0}")]
    Generic(String),
}

pub type Result<T> = core::result::Result<T, ParserError>;
