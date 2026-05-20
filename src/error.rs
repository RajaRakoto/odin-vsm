//! Shared error types for odin.
//!
//! Uses [`thiserror`] for ergonomic error definitions.

use thiserror::Error;

/// Application-wide error type.
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration or environment variable errors.
    #[error("Config error: {0}")]
    Config(String),

    /// I/O errors.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON errors.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// HTTP / network errors.
    #[error("Network error: {0}")]
    Network(String),

    /// Docker command errors.
    #[error("Docker error: {0}")]
    Docker(String),

    /// Validation errors.
    #[error("Validation error: {0}")]
    Validation(String),

    /// Catch-all.
    #[error("{0}")]
    Other(String),
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }
    pub fn docker(msg: impl Into<String>) -> Self {
        Self::Docker(msg.into())
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}
impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Self::Other(s.to_owned())
    }
}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(e.to_string())
    }
}
