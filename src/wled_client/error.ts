// src/wled_client/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WledError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid device address: {0}")]
    InvalidAddress(String),

    #[error("WLED API error: {status} - {message}")]
    Api { status: u16, message: String },

    #[error("Device not found: {address}")]
    DeviceNotFound(String),

    #[error("Timeout exceeded")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, WledError>;
