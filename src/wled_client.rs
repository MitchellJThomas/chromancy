// src/wled_client.rs
use reqwest::Client;
use std::time::Duration;

pub struct WledClient {
    client: Client,
    base_url: String,
    timeout: Duration,
}

pub struct WledClientBuilder {
    address: String,
    timeout: Duration,
    api_key: Option<String>,
}

impl WledClient {
    /// Simple constructor for quick use
    pub fn new(address: impl Into<String>) -> Result<Self> {
        Self::builder(address).build()
    }

    /// Builder for advanced configuration
    pub fn builder(address: impl Into<String>) -> WledClientBuilder {
        WledClientBuilder {
            address: address.into(),
            timeout: Duration::from_secs(5),
            api_key: None,
        }
    }
}

impl WledClientBuilder {
    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = duration;
        self
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn build(self) -> Result<WledClient> {
        let address = self.address.trim().trim_end_matches('/');
        
        // Validate it looks like an IP or hostname
        if address.is_empty() || address.starts_with("http") {
            return Err(WledError::InvalidAddress(
                "Address should be IP or hostname, not a full URL".into()
            ));
        }

        let base_url = format!("http://{}/json", address);
        
        let client = Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(WledError::Network)?;

        Ok(WledClient {
            client,
            base_url,
            timeout: self.timeout,
        })
    }
}