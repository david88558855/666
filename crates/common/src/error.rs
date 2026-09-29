//! Domain errors shared across crates.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommonError {
    #[error("invalid id: {0}")]
    InvalidId(String),

    #[error("not found: {0}")]
    NotFound(&'static str),

    #[error("forbidden")]
    Forbidden,

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("validation: {0}")]
    Validation(String),
}