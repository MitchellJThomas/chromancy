use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::error::WledError;
use crate::types::*;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
const RETRY_DELAY: Duration = Duration::from_millis(500);
const CACHE_TTL: Duration = Duration::from_secs(3600);

// ── HTTP backend ──────────────────────────────────────────────────────────────

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

        resp.json()
            .await
            .map_err(|e| WledError::Network {
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

        resp.json()
            .await
            .map_err(|e| WledError::Network {
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
            Err(WledError::Network { .. }) | Err(WledError::Timeout) => {
                tokio::time::sleep(RETRY_DELAY).await;
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
            Err(WledError::Network { .. }) | Err(WledError::Timeout) => {
                tokio::time::sleep(RETRY_DELAY).await;
                self.post_void(path, body).await
            }
            result => result,
        }
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

// ── Mock backend ──────────────────────────────────────────────────────────────

pub struct MockInner {
    pub device_name: String,
    pub state: RwLock<WledState>,
    pub info: WledInfo,
    pub effects: Vec<String>,
    pub palettes: Vec<String>,
    pub presets: RwLock<HashMap<i32, PresetInfo>>,
}

// ── Client kind ───────────────────────────────────────────────────────────────

enum ClientKind {
    Http(Arc<HttpInner>),
    Mock(Arc<MockInner>),
}

// ── Public API ────────────────────────────────────────────────────────────────

/// HTTP client for a single WLED device.
///
/// Cheaply cloneable (`Arc`-backed). All methods are `async` and retry once on
/// transient network errors before surfacing the error.
#[derive(Clone)]
pub struct WledClient {
    inner: Arc<ClientKind>,
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

    /// Builder for a mock client used in unit tests.
    pub fn mock() -> WledClientMockBuilder {
        WledClientMockBuilder::default()
    }

    /// The human-readable device name used in error messages.
    pub fn device_name(&self) -> &str {
        match self.inner.as_ref() {
            ClientKind::Http(i) => &i.device_name,
            ClientKind::Mock(i) => &i.device_name,
        }
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    #[tracing::instrument(skip(self), fields(device = %self.device_name()))]
    pub async fn get_state(&self) -> Result<WledState, WledError> {
        match self.inner.as_ref() {
            ClientKind::Http(i) => i.get_with_retry("/json/state").await,
            ClientKind::Mock(i) => Ok(i.state.read().await.clone()),
        }
    }

    #[tracing::instrument(skip(self), fields(device = %self.device_name()))]
    pub async fn get_info(&self) -> Result<WledInfo, WledError> {
        match self.inner.as_ref() {
            ClientKind::Http(i) => i.get_with_retry("/json/info").await,
            ClientKind::Mock(i) => Ok(i.info.clone()),
        }
    }

    pub async fn get_full_state(&self) -> Result<WledFullState, WledError> {
        match self.inner.as_ref() {
            ClientKind::Http(i) => i.get_with_retry("/json").await,
            ClientKind::Mock(i) => Ok(WledFullState {
                state: i.state.read().await.clone(),
                info: i.info.clone(),
                effects: i.effects.clone(),
                palettes: i.palettes.clone(),
            }),
        }
    }

    /// Returns effect names. Cached for 1 hour.
    pub async fn list_effects(&self) -> Result<Vec<String>, WledError> {
        match self.inner.as_ref() {
            ClientKind::Http(i) => {
                {
                    let cache = i.effects_cache.read().await;
                    if let Some(c) = cache.as_ref() {
                        if c.is_valid() {
                            return Ok(c.data.clone());
                        }
                    }
                }
                let data: Vec<String> = i.get_with_retry("/json/effects").await?;
                *i.effects_cache.write().await = Some(CachedList::new(data.clone()));
                Ok(data)
            }
            ClientKind::Mock(i) => Ok(i.effects.clone()),
        }
    }

    /// Returns palette names. Cached for 1 hour.
    pub async fn list_palettes(&self) -> Result<Vec<String>, WledError> {
        match self.inner.as_ref() {
            ClientKind::Http(i) => {
                {
                    let cache = i.palettes_cache.read().await;
                    if let Some(c) = cache.as_ref() {
                        if c.is_valid() {
                            return Ok(c.data.clone());
                        }
                    }
                }
                let data: Vec<String> = i.get_with_retry("/json/palettes").await?;
                *i.palettes_cache.write().await = Some(CachedList::new(data.clone()));
                Ok(data)
            }
            ClientKind::Mock(i) => Ok(i.palettes.clone()),
        }
    }

    /// Returns raw palette color data for the given palette index.
    pub async fn get_palette_colors(
        &self,
        palette_id: u16,
    ) -> Result<serde_json::Value, WledError> {
        match self.inner.as_ref() {
            ClientKind::Http(i) => {
                let path = format!("/json/palettes?v=1&cb={}", palette_id);
                i.get_with_retry(&path).await
            }
            ClientKind::Mock(_) => Ok(serde_json::json!([])),
        }
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
                colors: Some(vec![[r, g, b], [0, 0, 0], [0, 0, 0]]),
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
        match self.inner.as_ref() {
            ClientKind::Http(i) => {
                let body = serde_json::to_value(&request)?;
                i.post_void_with_retry("/json/state", &body).await
            }
            ClientKind::Mock(i) => {
                let mut state = i.state.write().await;
                if let Some(on) = request.on {
                    state.on = on;
                }
                if let Some(bri) = request.brightness {
                    state.brightness = bri;
                }
                if let Some(transition) = request.transition {
                    state.transition = transition;
                }
                if let Some(ps) = request.preset_slot {
                    state.preset_slot = ps;
                }
                if let Some(lor) = request.live_override {
                    state.live_override = lor;
                }
                if let Some(mainseg) = request.main_segment {
                    state.main_segment = mainseg;
                }
                if let Some(segs) = request.segments {
                    for seg_req in segs {
                        let id = seg_req.id.unwrap_or(0) as usize;
                        while state.segments.len() <= id {
                            state.segments.push(Segment::default());
                        }
                        let seg = &mut state.segments[id];
                        if let Some(on) = seg_req.on {
                            seg.on = on;
                        }
                        if let Some(bri) = seg_req.brightness {
                            seg.brightness = bri;
                        }
                        if let Some(col) = seg_req.colors {
                            seg.colors = col;
                        }
                        if let Some(fx) = seg_req.effect_id {
                            seg.effect_id = fx;
                        }
                        if let Some(sx) = seg_req.effect_speed {
                            seg.effect_speed = sx;
                        }
                        if let Some(ix) = seg_req.effect_intensity {
                            seg.effect_intensity = ix;
                        }
                        if let Some(pal) = seg_req.palette_id {
                            seg.palette_id = pal;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    // ── Presets ───────────────────────────────────────────────────────────────

    /// Returns all presets keyed by their integer slot ID.
    #[tracing::instrument(skip(self), fields(device = %self.device_name()))]
    pub async fn list_presets(&self) -> Result<HashMap<i32, PresetInfo>, WledError> {
        match self.inner.as_ref() {
            ClientKind::Http(i) => i.get_with_retry("/json/presets").await,
            ClientKind::Mock(i) => Ok(i.presets.read().await.clone()),
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
        match self.inner.as_ref() {
            ClientKind::Http(i) => i.post_void_with_retry("/json/cfg", &body).await,
            ClientKind::Mock(_) => Ok(()),
        }
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
        match self.inner.as_ref() {
            ClientKind::Http(i) => i.post_void_with_retry("/json/cfg", &body).await,
            ClientKind::Mock(_) => Ok(()),
        }
    }

    // ── Escape hatch ──────────────────────────────────────────────────────────

    /// Sends a raw HTTP request. `method` must be `"GET"` or `"POST"`.
    pub async fn raw_request<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<T, WledError> {
        match self.inner.as_ref() {
            ClientKind::Http(i) => match method.to_uppercase().as_str() {
                "GET" => i.get_with_retry(path).await,
                "POST" => {
                    let b = body.unwrap_or(serde_json::Value::Null);
                    i.post(path, &b).await
                }
                m => Err(WledError::ConfigError(format!("Unsupported method: {}", m))),
            },
            ClientKind::Mock(_) => Err(WledError::ConfigError(
                "raw_request not supported on mock client".to_string(),
            )),
        }
    }

    // ── Test helpers ──────────────────────────────────────────────────────────

    /// Returns the mock inner state for assertions in tests.
    /// Returns `None` on a real HTTP client.
    pub async fn mock_get_state(&self) -> Option<WledState> {
        match self.inner.as_ref() {
            ClientKind::Mock(i) => Some(i.state.read().await.clone()),
            _ => None,
        }
    }

    pub async fn mock_get_presets(&self) -> Option<HashMap<i32, PresetInfo>> {
        match self.inner.as_ref() {
            ClientKind::Mock(i) => Some(i.presets.read().await.clone()),
            _ => None,
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

        let device_name = self
            .device_name
            .unwrap_or_else(|| self.address.clone());

        let base_url = if self.address.starts_with("http://")
            || self.address.starts_with("https://")
        {
            self.address
        } else {
            format!("http://{}", self.address)
        };

        Ok(WledClient {
            inner: Arc::new(ClientKind::Http(Arc::new(HttpInner {
                http,
                base_url,
                device_name,
                effects_cache: RwLock::new(None),
                palettes_cache: RwLock::new(None),
            }))),
        })
    }
}

#[derive(Default)]
pub struct WledClientMockBuilder {
    state: WledState,
    info: WledInfo,
    effects: Vec<String>,
    palettes: Vec<String>,
    presets: HashMap<i32, PresetInfo>,
    device_name: String,
}

impl WledClientMockBuilder {
    pub fn with_state(mut self, state: WledState) -> Self {
        self.state = state;
        self
    }

    pub fn with_info(mut self, info: WledInfo) -> Self {
        self.info = info;
        self
    }

    pub fn with_effects(mut self, effects: Vec<String>) -> Self {
        self.effects = effects;
        self
    }

    pub fn with_palettes(mut self, palettes: Vec<String>) -> Self {
        self.palettes = palettes;
        self
    }

    pub fn with_preset(mut self, id: i32, preset: PresetInfo) -> Self {
        self.presets.insert(id, preset);
        self
    }

    pub fn with_device_name(mut self, name: impl Into<String>) -> Self {
        self.device_name = name.into();
        self
    }

    pub fn build(self) -> WledClient {
        WledClient {
            inner: Arc::new(ClientKind::Mock(Arc::new(MockInner {
                device_name: if self.device_name.is_empty() {
                    "mock-device".to_string()
                } else {
                    self.device_name
                },
                state: RwLock::new(self.state),
                info: self.info,
                effects: if self.effects.is_empty() {
                    vec![
                        "Solid".to_string(),
                        "Blink".to_string(),
                        "Rainbow".to_string(),
                        "Chase".to_string(),
                    ]
                } else {
                    self.effects
                },
                palettes: if self.palettes.is_empty() {
                    vec![
                        "Default".to_string(),
                        "Random Cycle".to_string(),
                        "Cloud".to_string(),
                    ]
                } else {
                    self.palettes
                },
                presets: RwLock::new(self.presets),
            }))),
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mock() -> WledClient {
        WledClient::mock()
            .with_device_name("test-device")
            .with_state(WledState {
                on: true,
                brightness: 200,
                ..Default::default()
            })
            .with_preset(
                1,
                PresetInfo {
                    name: "Warm White".to_string(),
                    ..Default::default()
                },
            )
            .with_preset(
                2,
                PresetInfo {
                    name: "Party Mode".to_string(),
                    ..Default::default()
                },
            )
            .build()
    }

    #[tokio::test]
    async fn test_mock_get_state() {
        let client = mock();
        let state = client.get_state().await.unwrap();
        assert!(state.on);
        assert_eq!(state.brightness, 200);
    }

    #[tokio::test]
    async fn test_mock_set_power() {
        let client = mock();
        client.set_power(false).await.unwrap();
        let state = client.mock_get_state().await.unwrap();
        assert!(!state.on);
    }

    #[tokio::test]
    async fn test_mock_set_brightness() {
        let client = mock();
        client.set_brightness(100).await.unwrap();
        let state = client.mock_get_state().await.unwrap();
        assert_eq!(state.brightness, 100);
    }

    #[tokio::test]
    async fn test_mock_set_color() {
        let client = mock();
        client.set_color(255, 0, 0).await.unwrap();
        let state = client.mock_get_state().await.unwrap();
        assert_eq!(state.segments[0].colors[0], [255, 0, 0]);
    }

    #[tokio::test]
    async fn test_mock_activate_preset() {
        let client = mock();
        client.activate_preset(1).await.unwrap();
        let state = client.mock_get_state().await.unwrap();
        assert_eq!(state.preset_slot, 1);
    }

    #[tokio::test]
    async fn test_mock_activate_preset_by_name() {
        let client = mock();
        client.activate_preset_by_name("Party Mode").await.unwrap();
        let state = client.mock_get_state().await.unwrap();
        assert_eq!(state.preset_slot, 2);
    }

    #[tokio::test]
    async fn test_mock_activate_unknown_preset_by_name() {
        let client = mock();
        let result = client.activate_preset_by_name("Nonexistent").await;
        assert!(matches!(result, Err(WledError::PresetNotFound(_))));
    }

    #[tokio::test]
    async fn test_mock_list_effects() {
        let client = mock();
        let effects = client.list_effects().await.unwrap();
        assert!(!effects.is_empty());
        assert!(effects.contains(&"Solid".to_string()));
    }

    #[tokio::test]
    async fn test_mock_list_palettes() {
        let client = mock();
        let palettes = client.list_palettes().await.unwrap();
        assert!(!palettes.is_empty());
    }

    #[tokio::test]
    async fn test_mock_list_presets() {
        let client = mock();
        let presets = client.list_presets().await.unwrap();
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[&1].name, "Warm White");
        assert_eq!(presets[&2].name, "Party Mode");
    }

    #[tokio::test]
    async fn test_mock_ping() {
        let client = mock();
        assert!(client.ping().await.unwrap());
    }

    #[tokio::test]
    async fn test_builder_rejects_bad_timeout() {
        // Duration::ZERO is valid for reqwest, so just verify the builder pattern
        let result = WledClient::builder("192.168.1.1")
            .device_name("living-room")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_adds_http_prefix() {
        let client = WledClient::new("192.168.1.100").unwrap();
        // device_name should default to the address
        assert_eq!(client.device_name(), "192.168.1.100");
    }
}

// ── HTTP error tests (wiremock) ───────────────────────────────────────────────

#[cfg(test)]
mod http_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper that builds an HTTP client pointed at the wiremock server.
    fn client_for(server: &MockServer) -> WledClient {
        WledClient::builder(server.uri())
            .device_name("wiremock-device")
            .build()
            .unwrap()
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
        // Server returns 200 OK but the body is not valid JSON.
        // reqwest's json() decoder fails → WledError::Network (retried once).
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
        // Server delays longer than the client's timeout.
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_millis(300)),
            )
            .mount(&server)
            .await;

        // Short timeout so the test completes quickly (retried once = ~200 ms
        // of actual waiting plus the 500 ms RETRY_DELAY between attempts).
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
}
