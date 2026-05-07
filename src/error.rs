use thiserror::Error;

#[derive(Debug, Error)]
pub enum WledError {
    #[error("Network error for device '{device}': {source}")]
    Network {
        device: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("API error for device '{device}': HTTP {status} - {message}")]
    Api {
        device: String,
        status: u16,
        message: String,
    },

    #[error("Device not found: '{0}'")]
    DeviceNotFound(String),

    #[error("Preset not found: '{0}'")]
    PresetNotFound(String),

    #[error("Invalid channel {channel} for device '{device}': max is {max_channels}")]
    InvalidChannel {
        device: String,
        channel: u8,
        max_channels: u8,
    },

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Request timed out")]
    Timeout,

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
