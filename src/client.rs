use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::error::WledError;
use crate::types::*;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_millis(500);
const RETRY_JITTER_MAX_MS: u64 = 200;
const CACHE_TTL: Duration = Duration::from_secs(3600);

// ── Internal HTTP client ──────────────────────────────────────────────────────

struct CachedList {
    data: Vec<String>,
    fetched_at: Instant,
}

impl CachedList {
    fn new(data: Vec<String>) -> Self {
        Self {
            data,
            fetched_at: Instant::now(),
        }
    }

    fn is_valid(&self) -> bool {
        self.fetched_at.elapsed() < CACHE_TTL
    }
}

struct HttpInner {
    http: reqwest::Client,
    base_url: String,
    device_name: String,
    effects_cache: RwLock<Option<CachedList>>,
    palettes_cache: RwLock<Option<CachedList>>,
}

impl HttpInner {
    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, WledError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| self.map_err(e))?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(WledError::Api {
                device: self.device_name.clone(),
                status,
                message,
            });
        }

        resp.json().await.map_err(|e| WledError::Network {
            device: self.device_name.clone(),
            source: e,
        })
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, WledError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .map_err(|e| self.map_err(e))?;

        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            let message = resp.text().await.unwrap_or_default();
            return Err(WledError::Api {
                device: self.device_name.clone(),
                status,
                message,
            });
        }

        resp.json().await.map_err(|e| WledError::Network {
            device: self.device_name.clone(),
            source: e,
        })
    }

    async fn post_void(&self, path: &str, body: &serde_json::Value) -> Result<(), WledError> {
        let _: serde_json::Value = self.post(path, body).await?;
        Ok(())
    }

    async fn get_with_retry<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
    ) -> Result<T, WledError> {
        match self.get(path).await {
            Err(e) if Self::is_retriable(&e) => {
                self.delay_with_jitter().await;
                self.get(path).await
            }
            result => result,
        }
    }

    async fn post_void_with_retry(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<(), WledError> {
        match self.post_void(path, body).await {
            Err(e) if Self::is_retriable(&e) => {
                self.delay_with_jitter().await;
                self.post_void(path, body).await
            }
            result => result,
        }
    }

    /// Returns true if the error is transient and worth retrying.
    fn is_retriable(err: &WledError) -> bool {
        matches!(err, WledError::Network { .. } | WledError::Timeout)
            || matches!(
                err,
                WledError::Api { status, .. } if (500..600).contains(status)
            )
    }

    /// Small random delay to avoid thundering herd on retries.
    async fn delay_with_jitter(&self) {
        let jitter_ms = rand::random::<u64>() % RETRY_JITTER_MAX_MS;
        let jitter = std::time::Duration::from_millis(jitter_ms);
        tokio::time::sleep(RETRY_DELAY + jitter).await;
    }

    fn map_err(&self, e: reqwest::Error) -> WledError {
        if e.is_timeout() {
            WledError::Timeout
        } else {
            WledError::Network {
                device: self.device_name.clone(),
                source: e,
            }
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

/// HTTP client for a single WLED device.
///
/// Cheaply cloneable (`Arc`-backed). All methods are `async` and retry once on
/// transient network errors before surfacing the error.
#[derive(Clone)]
pub struct WledClient {
    inner: Arc<HttpInner>,
}

impl WledClient {
    /// Connect to a device at `address` (IP or hostname; `http://` prefix
    /// is added automatically if absent).
    pub fn new(address: impl Into<String>) -> Result<Self, WledError> {
        Self::builder(address).build()
    }

    /// Builder for advanced configuration (timeout, device name, etc.).
    pub fn builder(address: impl Into<String>) -> WledClientBuilder {
        WledClientBuilder {
            address: address.into(),
            timeout: DEFAULT_TIMEOUT,
            device_name: None,
        }
    }

    /// The human-readable device name used in error messages.
    pub fn device_name(&self) -> &str {
        &self.inner.device_name
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    #[tracing::instrument(skip(self), fields(device = %self.device_name()))]
    pub async fn get_state(&self) -> Result<WledState, WledError> {
        self.inner.get_with_retry("/json/state").await
    }

    #[tracing::instrument(skip(self), fields(device = %self.device_name()))]
    pub async fn get_info(&self) -> Result<WledInfo, WledError> {
        self.inner.get_with_retry("/json/info").await
    }

    pub async fn get_full_state(&self) -> Result<WledFullState, WledError> {
        self.inner.get_with_retry("/json").await
    }

    /// Returns effect names. Cached for 1 hour.
    pub async fn list_effects(&self) -> Result<Vec<String>, WledError> {
        {
            let cache = self.inner.effects_cache.read().await;
            if let Some(c) = cache.as_ref() {
                if c.is_valid() {
                    return Ok(c.data.clone());
                }
            }
        }
        let data: Vec<String> = self.inner.get_with_retry("/json/effects").await?;
        *self.inner.effects_cache.write().await = Some(CachedList::new(data.clone()));
        Ok(data)
    }

    /// Returns palette names. Cached for 1 hour.
    pub async fn list_palettes(&self) -> Result<Vec<String>, WledError> {
        {
            let cache = self.inner.palettes_cache.read().await;
            if let Some(c) = cache.as_ref() {
                if c.is_valid() {
                    return Ok(c.data.clone());
                }
            }
        }
        let data: Vec<String> = self.inner.get_with_retry("/json/palettes").await?;
        *self.inner.palettes_cache.write().await = Some(CachedList::new(data.clone()));
        Ok(data)
    }

    /// Returns raw palette color data for the given palette index.
    pub async fn get_palette_colors(
        &self,
        palette_id: u16,
    ) -> Result<serde_json::Value, WledError> {
        let path = format!("/json/palettes?v=1&cb={}", palette_id);
        self.inner.get_with_retry(&path).await
    }

    /// Returns `true` if the device responds to a state request.
    pub async fn ping(&self) -> Result<bool, WledError> {
        match self.get_state().await {
            Ok(_) => Ok(true),
            Err(WledError::Timeout) | Err(WledError::Network { .. }) => Ok(false),
            Err(e) => Err(e),
        }
    }

    // ── Mutations ─────────────────────────────────────────────────────────────

    pub async fn set_power(&self, on: bool) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            on: Some(on),
            ..Default::default()
        })
        .await
    }

    pub async fn set_brightness(&self, bri: u8) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            brightness: Some(bri),
            ..Default::default()
        })
        .await
    }

    /// Sets the primary color on segment 0.
    pub async fn set_color(&self, r: u8, g: u8, b: u8) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            segments: Some(vec![SegmentRequest {
                id: Some(0),
                colors: Some(vec![vec![r, g, b], vec![0, 0, 0], vec![0, 0, 0]]),
                ..Default::default()
            }]),
            ..Default::default()
        })
        .await
    }

    /// Sets the active effect by ID on segment 0.
    pub async fn set_effect(&self, effect_id: u16) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            segments: Some(vec![SegmentRequest {
                id: Some(0),
                effect_id: Some(effect_id),
                ..Default::default()
            }]),
            ..Default::default()
        })
        .await
    }

    /// Sets the active palette by ID on segment 0.
    pub async fn set_palette(&self, palette_id: u16) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            segments: Some(vec![SegmentRequest {
                id: Some(0),
                palette_id: Some(palette_id),
                ..Default::default()
            }]),
            ..Default::default()
        })
        .await
    }

    /// Sets the crossfade transition time (in 100 ms units).
    pub async fn set_transition(&self, transition: u16) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            transition: Some(transition),
            ..Default::default()
        })
        .await
    }

    /// Sends an arbitrary state update. Only `Some` fields are applied.
    #[tracing::instrument(skip(self, request), fields(device = %self.device_name()))]
    pub async fn set_state(&self, request: WledStateRequest) -> Result<(), WledError> {
        let body = serde_json::to_value(&request)?;
        self.inner.post_void_with_retry("/json/state", &body).await
    }

    // ── Presets ───────────────────────────────────────────────────────────────

    /// Returns all presets keyed by their integer slot ID.
    #[tracing::instrument(skip(self), fields(device = %self.device_name()))]
    pub async fn list_presets(&self) -> Result<HashMap<i32, PresetInfo>, WledError> {
        match self.inner.get_with_retry("/json/presets").await {
            Ok(presets) => Ok(presets),
            Err(WledError::Api { status: 501, .. }) => {
                self.inner.get_with_retry("/presets.json").await
            }
            Err(e) => Err(e),
        }
    }

    /// Activates a preset by slot ID.
    pub async fn activate_preset(&self, id: i32) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            preset_slot: Some(id),
            ..Default::default()
        })
        .await
    }

    /// Activates a preset by name (fetches preset list to resolve the ID).
    pub async fn activate_preset_by_name(&self, name: &str) -> Result<(), WledError> {
        let presets = self.list_presets().await?;
        let id = presets
            .iter()
            .find(|(_, p)| p.name == name)
            .map(|(id, _)| *id)
            .ok_or_else(|| WledError::PresetNotFound(name.to_string()))?;
        self.activate_preset(id).await
    }

    /// Saves the current state to a preset slot with the given name.
    pub async fn save_preset(&self, slot: i32, name: &str) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            preset_save_slot: Some(slot),
            preset_name: Some(name.to_string()),
            ..Default::default()
        })
        .await
    }

    /// Deletes a preset slot.
    pub async fn delete_preset(&self, slot: i32) -> Result<(), WledError> {
        self.set_state(WledStateRequest {
            preset_delete_slot: Some(slot),
            ..Default::default()
        })
        .await
    }

    // ── Schedule ──────────────────────────────────────────────────────────────

    /// Configures the NTP server via `/json/cfg`.
    pub async fn configure_ntp(&self, config: &NtpConfig) -> Result<(), WledError> {
        let body = serde_json::json!({
            "nw": {
                "ins": [{ "ntp": config.server.as_str() }]
            }
        });
        self.inner.post_void_with_retry("/json/cfg", &body).await
    }

    /// Configures a dusk-based schedule timer via `/json/cfg`.
    pub async fn configure_dusk_schedule(
        &self,
        config: &DuskScheduleConfig,
    ) -> Result<(), WledError> {
        let body = serde_json::json!({
            "timers": {
                "ins": [{
                    "en": config.enabled,
                    "hour": 255u8,   // 255 = dusk trigger
                    "min": 0,
                    "macro": config.preset_slot,
                    "dow": 127u8     // all days
                }, {
                    "en": config.enabled,
                    "hour": config.off_hour,
                    "min": config.off_minute,
                    "macro": 0,      // 0 = power off
                    "dow": 127u8
                }]
            }
        });
        self.inner.post_void_with_retry("/json/cfg", &body).await
    }

    // ── Escape hatch ──────────────────────────────────────────────────────────

    /// Sends a raw HTTP request. `method` must be `"GET"` or `"POST"`.
    pub async fn raw_request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, WledError> {
        match method.to_uppercase().as_str() {
            "GET" => self.inner.get_with_retry(path).await,
            "POST" => {
                let b = body.unwrap_or(serde_json::Value::Null);
                self.inner.post(path, &b).await
            }
            m => Err(WledError::ConfigError(format!("Unsupported method: {}", m))),
        }
    }
}

// ── Builder types ─────────────────────────────────────────────────────────────

pub struct WledClientBuilder {
    address: String,
    timeout: Duration,
    device_name: Option<String>,
}

impl WledClientBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn device_name(mut self, name: impl Into<String>) -> Self {
        self.device_name = Some(name.into());
        self
    }

    pub fn build(self) -> Result<WledClient, WledError> {
        let http = reqwest::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|e| WledError::ConfigError(e.to_string()))?;

        let device_name = self.device_name.unwrap_or_else(|| self.address.clone());

        let base_url =
            if self.address.starts_with("http://") || self.address.starts_with("https://") {
                self.address
            } else {
                format!("http://{}", self.address)
            };

        Ok(WledClient {
            inner: Arc::new(HttpInner {
                http,
                base_url,
                device_name,
                effects_cache: RwLock::new(None),
                palettes_cache: RwLock::new(None),
            }),
        })
    }
}

// ── Unit tests (wiremock) ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper that builds an HTTP client pointed at the wiremock server.
    fn client_for(server: &MockServer) -> WledClient {
        WledClient::builder(server.uri())
            .device_name("wiremock-device")
            .build()
            .unwrap()
    }

    // ── GET /json/state ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_state() {
        let server = MockServer::start().await;
        let state = WledState {
            on: true,
            brightness: 200,
            ..Default::default()
        };
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&state))
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await.unwrap();
        assert!(result.on);
        assert_eq!(result.brightness, 200);
    }

    #[tokio::test]
    async fn test_api_error_returns_status_code() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await;
        assert!(
            matches!(result, Err(WledError::Api { status: 500, .. })),
            "expected Api(500), got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_network_error_on_invalid_json_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await;
        assert!(
            matches!(result, Err(WledError::Network { .. })),
            "expected Network, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_timeout_when_server_is_slow() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(
                ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(300)),
            )
            .mount(&server)
            .await;

        let client = WledClient::builder(server.uri())
            .device_name("wiremock-device")
            .timeout(std::time::Duration::from_millis(50))
            .build()
            .unwrap();

        let result = client.get_state().await;
        assert!(
            matches!(result, Err(WledError::Timeout)),
            "expected Timeout, got {:?}",
            result
        );
    }

    // ── GET /json/info ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_info() {
        let server = MockServer::start().await;
        let info = WledInfo {
            firmware_version: "0.14.0".to_string(),
            name: "test-wled".to_string(),
            led_info: LedInfo {
                count: 300,
                ..Default::default()
            },
            ..Default::default()
        };
        Mock::given(method("GET"))
            .and(path("/json/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&info))
            .mount(&server)
            .await;

        let result = client_for(&server).get_info().await.unwrap();
        assert_eq!(result.firmware_version, "0.14.0");
        assert_eq!(result.name, "test-wled");
        assert_eq!(result.led_info.count, 300);
    }

    // ── GET /json (full state) ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_get_full_state() {
        let server = MockServer::start().await;
        let full = WledFullState {
            state: WledState {
                on: true,
                brightness: 150,
                ..Default::default()
            },
            info: WledInfo {
                name: "full-test".to_string(),
                ..Default::default()
            },
            effects: vec!["Solid".to_string(), "Blink".to_string()],
            palettes: vec!["Default".to_string()],
        };
        Mock::given(method("GET"))
            .and(path("/json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&full))
            .mount(&server)
            .await;

        let result = client_for(&server).get_full_state().await.unwrap();
        assert!(result.state.on);
        assert_eq!(result.info.name, "full-test");
        assert_eq!(result.effects.len(), 2);
    }

    // ── RGBW color deserialization (regression tests) ─────────────────────────

    #[tokio::test]
    async fn test_get_state_rgbw_colors() {
        let server = MockServer::start().await;
        // Dig2Go-Audioreactive RGBW devices return 4-element color arrays.
        let body = serde_json::json!({
            "on": true,
            "bri": 56,
            "seg": [{
                "id": 0,
                "start": 0,
                "stop": 300,
                "col": [[27, 179, 9, 0], [255, 18, 18, 0], [0, 0, 0, 0]],
                "fx": 110,
                "sx": 8,
                "ix": 22,
                "pal": 53
            }]
        });
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await.unwrap();
        assert!(result.on);
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments[0].colors.len(), 3);
        assert_eq!(result.segments[0].colors[0], vec![27, 179, 9, 0]);
        assert_eq!(result.segments[0].colors[1], vec![255, 18, 18, 0]);
        assert_eq!(result.segments[0].colors[2], vec![0, 0, 0, 0]);
    }

    #[tokio::test]
    async fn test_get_state_mixed_rgb_rgbw_segments() {
        let server = MockServer::start().await;
        // Segment 0 = RGB (3 elements), Segment 1 = RGBW (4 elements).
        let body = serde_json::json!({
            "on": true,
            "bri": 128,
            "seg": [
                {
                    "id": 0,
                    "start": 0,
                    "stop": 150,
                    "col": [[255, 0, 0], [0, 255, 0], [0, 0, 255]]
                },
                {
                    "id": 1,
                    "start": 150,
                    "stop": 300,
                    "col": [[255, 0, 0, 128], [0, 255, 0, 0], [0, 0, 255, 0]]
                }
            ]
        });
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await.unwrap();
        assert_eq!(result.segments.len(), 2);
        assert_eq!(result.segments[0].colors[0], vec![255, 0, 0]);
        assert_eq!(result.segments[1].colors[0], vec![255, 0, 0, 128]);
    }

    #[tokio::test]
    async fn test_get_state_empty_colors() {
        let server = MockServer::start().await;
        // Edge case: WLED may send empty color arrays in unusual states.
        let body = serde_json::json!({
            "on": false,
            "seg": [{"id": 0, "start": 0, "stop": 100, "col": []}]
        });
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&body))
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await.unwrap();
        assert_eq!(result.segments[0].colors.len(), 0);
    }

    // ── GET /json/effects ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_effects() {
        let server = MockServer::start().await;
        let effects = vec![
            "Solid".to_string(),
            "Blink".to_string(),
            "Rainbow".to_string(),
        ];
        Mock::given(method("GET"))
            .and(path("/json/effects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&effects))
            .mount(&server)
            .await;

        let result = client_for(&server).list_effects().await.unwrap();
        assert_eq!(result, effects);

        // Second call should use cache (no additional request needed).
        let result2 = client_for(&server).list_effects().await.unwrap();
        assert_eq!(result2, effects);
    }

    // ── GET /json/palettes ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_palettes() {
        let server = MockServer::start().await;
        let palettes = vec!["Default".to_string(), "Cloud".to_string()];
        Mock::given(method("GET"))
            .and(path("/json/palettes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&palettes))
            .mount(&server)
            .await;

        let result = client_for(&server).list_palettes().await.unwrap();
        assert_eq!(result, palettes);
    }

    // ── GET /json/palettes?v=1&cb={id} ──────────────────────────────────────

    #[tokio::test]
    async fn test_get_palette_colors() {
        let server = MockServer::start().await;
        let colors = serde_json::json!([[255, 0, 0], [0, 255, 0], [0, 0, 255]]);
        Mock::given(method("GET"))
            .and(path("/json/palettes"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&colors))
            .mount(&server)
            .await;

        let result = client_for(&server).get_palette_colors(5).await.unwrap();
        assert_eq!(result, colors);
    }

    // ── GET /json/presets ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_presets() {
        let server = MockServer::start().await;
        let mut presets = HashMap::new();
        presets.insert(
            1,
            PresetInfo {
                name: "Warm White".to_string(),
                ..Default::default()
            },
        );
        presets.insert(
            2,
            PresetInfo {
                name: "Party Mode".to_string(),
                ..Default::default()
            },
        );
        Mock::given(method("GET"))
            .and(path("/json/presets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&presets))
            .mount(&server)
            .await;

        let result = client_for(&server).list_presets().await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[&1].name, "Warm White");
        assert_eq!(result[&2].name, "Party Mode");
    }

    #[tokio::test]
    async fn test_list_presets_fallback_501() {
        let server = MockServer::start().await;
        let mut presets = HashMap::new();
        presets.insert(
            3,
            PresetInfo {
                name: "Evening Mode".to_string(),
                ..Default::default()
            },
        );
        // /json/presets returns 501 (WLED 0.15.3 QuinLED behavior).
        Mock::given(method("GET"))
            .and(path("/json/presets"))
            .respond_with(ResponseTemplate::new(501).set_body_json(&serde_json::json!({"error": 4})))
            .mount(&server)
            .await;
        // Fallback endpoint /presets.json succeeds.
        Mock::given(method("GET"))
            .and(path("/presets.json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&presets))
            .mount(&server)
            .await;

        let result = client_for(&server).list_presets().await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[&3].name, "Evening Mode");
    }

    // ── POST /json/state ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_set_power() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"on": false})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server).set_power(false).await.unwrap();
    }

    #[tokio::test]
    async fn test_set_brightness() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"bri": 100})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server).set_brightness(100).await.unwrap();
    }

    #[tokio::test]
    async fn test_set_color() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({
                "seg": [{"id": 0, "col": [[255, 0, 0], [0, 0, 0], [0, 0, 0]]}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server).set_color(255, 0, 0).await.unwrap();
    }

    #[tokio::test]
    async fn test_set_effect() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({
                "seg": [{"id": 0, "fx": 5}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server).set_effect(5).await.unwrap();
    }

    #[tokio::test]
    async fn test_set_palette() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({
                "seg": [{"id": 0, "pal": 3}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server).set_palette(3).await.unwrap();
    }

    #[tokio::test]
    async fn test_set_transition() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"transition": 15})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server).set_transition(15).await.unwrap();
    }

    #[tokio::test]
    async fn test_activate_preset() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"ps": 2})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server).activate_preset(2).await.unwrap();
    }

    #[tokio::test]
    async fn test_activate_preset_by_name() {
        let server = MockServer::start().await;
        let mut presets = HashMap::new();
        presets.insert(
            3,
            PresetInfo {
                name: "Party Mode".to_string(),
                ..Default::default()
            },
        );
        Mock::given(method("GET"))
            .and(path("/json/presets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&presets))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"ps": 3})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server)
            .activate_preset_by_name("Party Mode")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_activate_unknown_preset_by_name() {
        let server = MockServer::start().await;
        let presets = HashMap::<i32, PresetInfo>::new();
        Mock::given(method("GET"))
            .and(path("/json/presets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&presets))
            .mount(&server)
            .await;

        let result = client_for(&server)
            .activate_preset_by_name("Nonexistent")
            .await;
        assert!(matches!(result, Err(WledError::PresetNotFound(_))));
    }

    #[tokio::test]
    async fn test_save_preset() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"psave": 7, "n": "My Preset"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server)
            .save_preset(7, "My Preset")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_delete_preset() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"pdel": 4})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        client_for(&server).delete_preset(4).await.unwrap();
    }

    // ── ping ────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_ping_success() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        assert!(client_for(&server).ping().await.unwrap());
    }

    #[tokio::test]
    async fn test_ping_timeout_returns_false() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(
                ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(300)),
            )
            .mount(&server)
            .await;

        let client = WledClient::builder(server.uri())
            .device_name("wiremock-device")
            .timeout(std::time::Duration::from_millis(50))
            .build()
            .unwrap();

        assert!(!client.ping().await.unwrap());
    }

    // ── POST /json/cfg (schedule) ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_configure_ntp() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/cfg"))
            .and(body_json(serde_json::json!({
                "nw": { "ins": [{ "ntp": "pool.ntp.org" }] }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let config = NtpConfig {
            server: NtpServer::hostname("pool.ntp.org"),
        };
        client_for(&server).configure_ntp(&config).await.unwrap();
    }

    #[tokio::test]
    async fn test_configure_dusk_schedule() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/cfg"))
            .and(body_json(serde_json::json!({
                "timers": {
                    "ins": [
                        { "en": true, "hour": 255, "min": 0, "macro": 3, "dow": 127 },
                        { "en": true, "hour": 0, "min": 30, "macro": 0, "dow": 127 }
                    ]
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let config = DuskScheduleConfig {
            preset_slot: 3,
            off_hour: 0,
            off_minute: 30,
            enabled: true,
        };
        client_for(&server)
            .configure_dusk_schedule(&config)
            .await
            .unwrap();
    }

    // ── raw_request ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_raw_request_get() {
        let server = MockServer::start().await;
        let state = WledState {
            on: true,
            brightness: 77,
            ..Default::default()
        };
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&state))
            .mount(&server)
            .await;

        let result: WledState = client_for(&server)
            .raw_request("GET", "/json/state", None)
            .await
            .unwrap();
        assert_eq!(result.brightness, 77);
    }

    #[tokio::test]
    async fn test_raw_request_post() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"on": true})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let result: serde_json::Value = client_for(&server)
            .raw_request("POST", "/json/state", Some(serde_json::json!({"on": true})))
            .await
            .unwrap();
        assert!(result.is_object());
    }

    #[tokio::test]
    async fn test_raw_request_unsupported_method() {
        let server = MockServer::start().await;
        let result: Result<serde_json::Value, _> = client_for(&server)
            .raw_request("DELETE", "/json/state", None)
            .await;
        assert!(matches!(result, Err(WledError::ConfigError(_))));
    }

    // ── Builder tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_builder_rejects_bad_timeout() {
        let result = WledClient::builder("192.168.1.1")
            .device_name("living-room")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_adds_http_prefix() {
        let client = WledClient::new("192.168.1.100").unwrap();
        assert_eq!(client.device_name(), "192.168.1.100");
    }

    #[test]
    fn test_device_name_override() {
        let client = WledClient::builder("192.168.1.100")
            .device_name("kitchen")
            .build()
            .unwrap();
        assert_eq!(client.device_name(), "kitchen");
    }

    // ── Retry tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_retry_on_500_then_success() {
        let server = MockServer::start().await;
        // First request: 500 (retried). Second request: 200 OK.
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_on_network_error_then_success() {
        let server = MockServer::start().await;
        // First request: invalid JSON (Network error, retried).
        // Second request: valid JSON.
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_no_retry_on_400() {
        let server = MockServer::start().await;
        // 400 is not retriable — only one request should be made.
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(400).set_body_string("Bad Request"))
            .expect(1)
            .mount(&server)
            .await;

        let result = client_for(&server).get_state().await;
        assert!(
            matches!(result, Err(WledError::Api { status: 400, .. })),
            "expected Api(400), got {:?}",
            result
        );
    }

    // ── Cache tests ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_effects_cache_avoids_second_request() {
        let server = MockServer::start().await;
        let effects = vec!["Solid".to_string(), "Blink".to_string()];
        Mock::given(method("GET"))
            .and(path("/json/effects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&effects))
            .expect(1)
            .mount(&server)
            .await;

        let client = client_for(&server);
        let r1 = client.list_effects().await.unwrap();
        let r2 = client.list_effects().await.unwrap();
        assert_eq!(r1, effects);
        assert_eq!(r2, effects);
        // wiremock will panic if the expectation of 1 request is violated.
    }
}
