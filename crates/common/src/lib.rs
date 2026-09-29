//! Shared types and helpers used across the gostc-rs workspace.

pub mod error;
pub mod ids;

pub use error::CommonError;
pub use ids::{NodeId, TunnelId, UserId};

/// Result alias used by all crates.
pub type Result<T> = std::result::Result<T, CommonError>;