use std::collections::HashMap;

use crate::client::WledClient;
use crate::error::WledError;
use crate::types::*;

// ── Internal device entry ─────────────────────────────────────────────────────

#[derive(Clone)]
struct DeviceEntry {
    name: String,
    client: WledClient,
    device_type: String,
}

// ── Public types ──────────────────────────────────────────────────────────────

/// Orchestrates a WLED leader device plus any number of followers.
///
/// All group-level control operations (power, brightness, preset, effect,
/// palette, color) are sent **only to the leader**. WLED's built-in UDP sync
/// then propagates the change to followers automatically.
///
/// Followers can still be accessed individually for troubleshooting or direct
/// control. Use [`check_sync_health`](WledSyncGroup::check_sync_health) to
/// detect drift and [`force_resync`](WledSyncGroup::force_resync) to correct it.
#[derive(Clone)]
pub struct WledSyncGroup {
    pub name: String,
    leader: DeviceEntry,
    followers: Vec<DeviceEntry>,
}

impl WledSyncGroup {
    /// Construct a group with a single leader device.
    pub fn new(
        name: impl Into<String>,
        leader_name: impl Into<String>,
        leader_client: WledClient,
        leader_device_type: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            leader: DeviceEntry {
                name: leader_name.into(),
                client: leader_client,
                device_type: leader_device_type.into(),
            },
            followers: Vec::new(),
        }
    }

    /// Add a follower device to the group.
    pub fn add_follower(
        &mut self,
        name: impl Into<String>,
        client: WledClient,
        device_type: impl Into<String>,
    ) {
        self.followers.push(DeviceEntry {
            name: name.into(),
            client,
            device_type: device_type.into(),
        });
    }

    // ── Device access ─────────────────────────────────────────────────────────

    /// Returns the leader client.
    pub fn leader(&self) -> &WledClient {
        &self.leader.client
    }

    /// Returns the leader's device name.
    pub fn leader_name(&self) -> &str {
        &self.leader.name
    }

    /// Returns the leader's device type string.
    pub fn leader_device_type(&self) -> &str {
        &self.leader.device_type
    }

    /// Returns the client for the named follower, if it exists.
    pub fn get_follower(&self, name: &str) -> Option<&WledClient> {
        self.followers
            .iter()
            .find(|f| f.name == name)
            .map(|f| &f.client)
    }

    /// Returns all follower `(name, client)` pairs.
    pub fn list_followers(&self) -> Vec<(&str, &WledClient)> {
        self.followers
            .iter()
            .map(|f| (f.name.as_str(), &f.client))
            .collect()
    }

    /// Returns any device (leader or follower) by name.
    pub fn get_device(&self, name: &str) -> Option<&WledClient> {
        if self.leader.name == name {
            return Some(&self.leader.client);
        }
        self.get_follower(name)
    }

    /// Returns all devices `(name, client)` — leader first, then followers.
    pub fn list_devices(&self) -> Vec<(&str, &WledClient)> {
        let mut out = vec![(self.leader.name.as_str(), &self.leader.client)];
        out.extend(self.list_followers());
        out
    }

    /// Total device count (leader + followers).
    pub fn device_count(&self) -> usize {
        1 + self.followers.len()
    }

    // ── Group operations (all via leader) ─────────────────────────────────────

    /// Turns the group on or off via the leader.
    pub async fn set_power(&self, on: bool) -> Result<(), WledError> {
        self.leader.client.set_power(on).await
    }

    /// Sets master brightness (0–255) via the leader.
    pub async fn set_brightness(&self, bri: u8) -> Result<(), WledError> {
        self.leader.client.set_brightness(bri).await
    }

    /// Activates a preset by name on the leader.
    ///
    /// WLED UDP sync propagates the change to followers automatically.
    #[tracing::instrument(skip(self), fields(group = %self.name, preset = %name))]
    pub async fn activate_preset(&self, name: &str) -> Result<(), WledError> {
        self.leader.client.activate_preset_by_name(name).await
    }

    /// Returns the leader's current state.
    pub async fn get_state(&self) -> Result<WledState, WledError> {
        self.leader.client.get_state().await
    }

    /// Lists presets available on the leader.
    pub async fn list_presets(&self) -> Result<HashMap<i32, PresetInfo>, WledError> {
        self.leader.client.list_presets().await
    }

    /// Sets the primary color on segment 0 of the leader.
    pub async fn set_color(&self, r: u8, g: u8, b: u8) -> Result<(), WledError> {
        self.leader.client.set_color(r, g, b).await
    }

    /// Sets the active effect by name on the leader's segment 0.
    ///
    /// Fetches the effect list (cached) to resolve the name to an ID.
    pub async fn set_effect(&self, effect_name: &str) -> Result<(), WledError> {
        let effects = self.leader.client.list_effects().await?;
        let id = effects
            .iter()
            .position(|e| e == effect_name)
            .ok_or_else(|| WledError::ConfigError(format!("Effect '{}' not found", effect_name)))?;
        self.leader.client.set_effect(id as u16).await
    }

    /// Sets the active palette by name on the leader's segment 0.
    ///
    /// Fetches the palette list (cached) to resolve the name to an ID.
    pub async fn set_palette(&self, palette_name: &str) -> Result<(), WledError> {
        let palettes = self.leader.client.list_palettes().await?;
        let id = palettes
            .iter()
            .position(|p| p == palette_name)
            .ok_or_else(|| {
                WledError::ConfigError(format!("Palette '{}' not found", palette_name))
            })?;
        self.leader.client.set_palette(id as u16).await
    }

    /// Sets the color on a specific output channel of the leader (1-indexed).
    ///
    /// Channels map to LED segments (channel 1 = segment 0, channel 2 = segment 1,
    /// etc.). Validates against the leader's actual segment count.
    pub async fn set_channel_color(
        &self,
        channel: u8,
        r: u8,
        g: u8,
        b: u8,
    ) -> Result<(), WledError> {
        if channel == 0 {
            return Err(WledError::InvalidChannel {
                device: self.leader.name.clone(),
                channel,
                max_channels: u8::MAX,
            });
        }
        let state = self.leader.client.get_state().await?;
        let max_channels = state.segments.len() as u8;
        if channel > max_channels {
            return Err(WledError::InvalidChannel {
                device: self.leader.name.clone(),
                channel,
                max_channels,
            });
        }
        self.leader
            .client
            .set_state(WledStateRequest {
                segments: Some(vec![SegmentRequest {
                    id: Some(channel - 1),
                    colors: Some(vec![[r, g, b], [0, 0, 0], [0, 0, 0]]),
                    ..Default::default()
                }]),
                ..Default::default()
            })
            .await
    }

    // ── Sync health ───────────────────────────────────────────────────────────

    #[tracing::instrument(skip(self), fields(group = %self.name))]
    /// Compares each follower's active preset against the leader's.
    /// Returns a report indicating which (if any) devices have drifted.
    pub async fn check_sync_health(&self) -> Result<SyncHealthReport, WledError> {
        let leader_state = self.leader.client.get_state().await?;
        let leader_preset = leader_state.preset_slot;

        // Query all followers concurrently.
        let mut handles = tokio::task::JoinSet::new();
        for follower in &self.followers {
            let client = follower.client.clone();
            let name = follower.name.clone();
            handles.spawn(async move { (name, client.get_state().await) });
        }

        let mut devices = Vec::new();
        let mut all_healthy = true;

        while let Some(res) = handles.join_next().await {
            let (device_name, state_result) = res.expect("task panicked");
            match state_result {
                Ok(state) => {
                    let healthy = state.preset_slot == leader_preset;
                    if !healthy {
                        all_healthy = false;
                    }
                    devices.push(DeviceSyncStatus {
                        device_name,
                        is_healthy: healthy,
                        leader_preset,
                        device_preset: state.preset_slot,
                    });
                }
                Err(e) => {
                    all_healthy = false;
                    devices.push(DeviceSyncStatus {
                        device_name: device_name.clone(),
                        is_healthy: false,
                        leader_preset,
                        device_preset: -1,
                    });
                    tracing::warn!(device = %device_name, error = %e, "Failed to get state for health check");
                }
            }
        }

        Ok(SyncHealthReport {
            group_name: self.name.clone(),
            healthy: all_healthy,
            devices,
        })
    }

    #[tracing::instrument(skip(self), fields(group = %self.name))]
    /// Forces all followers to match the leader's current state via direct HTTP
    /// (bypasses UDP sync). Useful when UDP sync is misconfigured or delayed.
    ///
    /// If the leader has an active preset, that preset is activated on each
    /// follower. Otherwise, the leader's full state is pushed directly.
    pub async fn force_resync(&self) -> Result<(), WledError> {
        if self.followers.is_empty() {
            return Ok(());
        }

        let leader_state = self.leader.client.get_state().await?;
        let mut errors: Vec<String> = Vec::new();

        if leader_state.preset_slot >= 0 {
            // Activate the same preset on each follower.
            let preset_id = leader_state.preset_slot;
            let mut handles = tokio::task::JoinSet::new();
            for follower in &self.followers {
                let client = follower.client.clone();
                let name = follower.name.clone();
                handles.spawn(async move { (name, client.activate_preset(preset_id).await) });
            }
            while let Some(res) = handles.join_next().await {
                let (name, result) = res.expect("task panicked");
                if let Err(e) = result {
                    errors.push(format!("{}: {}", name, e));
                }
            }
        } else {
            // No active preset — push the full state directly.
            let request = leader_state_to_request(&leader_state);
            let mut handles = tokio::task::JoinSet::new();
            for follower in &self.followers {
                let client = follower.client.clone();
                let name = follower.name.clone();
                let req = request.clone();
                handles.spawn(async move { (name, client.set_state(req).await) });
            }
            while let Some(res) = handles.join_next().await {
                let (name, result) = res.expect("task panicked");
                if let Err(e) = result {
                    errors.push(format!("{}: {}", name, e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(WledError::ConfigError(format!(
                "force_resync failed for {} device(s): {}",
                errors.len(),
                errors.join(", ")
            )))
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Converts a full WledState snapshot into an equivalent WledStateRequest so it
/// can be pushed to follower devices during force_resync.
fn leader_state_to_request(state: &WledState) -> WledStateRequest {
    WledStateRequest {
        on: Some(state.on),
        brightness: Some(state.brightness),
        transition: Some(state.transition),
        segments: Some(
            state
                .segments
                .iter()
                .enumerate()
                .map(|(i, seg)| SegmentRequest {
                    id: Some(i as u8),
                    on: Some(seg.on),
                    brightness: Some(seg.brightness),
                    colors: Some(seg.colors.clone()),
                    effect_id: Some(seg.effect_id),
                    effect_speed: Some(seg.effect_speed),
                    effect_intensity: Some(seg.effect_intensity),
                    palette_id: Some(seg.palette_id),
                    ..Default::default()
                })
                .collect(),
        ),
        ..Default::default()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Build a WledClient pointed at a wiremock server.
    fn client_for(server: &MockServer, name: &str) -> WledClient {
        WledClient::builder(server.uri())
            .device_name(name)
            .build()
            .unwrap()
    }

    /// Default leader state returned by wiremock.
    fn leader_state() -> WledState {
        WledState {
            on: true,
            brightness: 200,
            preset_slot: 3,
            segments: vec![Segment {
                id: 0,
                colors: vec![[255, 0, 0], [0, 0, 0], [0, 0, 0]],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// Default leader presets returned by wiremock.
    fn leader_presets() -> HashMap<i32, PresetInfo> {
        let mut m = HashMap::new();
        m.insert(
            3,
            PresetInfo {
                name: "Red Party".to_string(),
                ..Default::default()
            },
        );
        m
    }

    /// Default effect list.
    fn effects() -> Vec<String> {
        vec![
            "Solid".to_string(),
            "Blink".to_string(),
            "Rainbow".to_string(),
        ]
    }

    /// Set up wiremock stubs for a leader device.
    async fn mount_leader_stubs(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&leader_state()))
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path("/json/presets"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&leader_presets()))
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path("/json/effects"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&effects()))
            .mount(server)
            .await;
    }

    /// Set up wiremock stubs for a follower with the given preset.
    async fn mount_follower_stubs(server: &MockServer, preset: i32) {
        let state = WledState {
            preset_slot: preset,
            ..Default::default()
        };
        Mock::given(method("GET"))
            .and(path("/json/state"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&state))
            .mount(server)
            .await;
    }

    /// Build a sync group with wiremock-backed leader + two followers.
    async fn make_group() -> (WledSyncGroup, MockServer, MockServer, MockServer) {
        let leader_srv = MockServer::start().await;
        let f1_srv = MockServer::start().await;
        let f2_srv = MockServer::start().await;

        mount_leader_stubs(&leader_srv).await;
        mount_follower_stubs(&f1_srv, 3).await;
        mount_follower_stubs(&f2_srv, 99).await;

        let mut g = WledSyncGroup::new(
            "test-group",
            "leader-1",
            client_for(&leader_srv, "leader-1"),
            "DigQuad",
        );
        g.add_follower("follower-1", client_for(&f1_srv, "follower-1"), "DigUno");
        g.add_follower("follower-2", client_for(&f2_srv, "follower-2"), "Dig2Go");

        (g, leader_srv, f1_srv, f2_srv)
    }

    #[tokio::test]
    async fn test_group_get_state() {
        let (g, _ls, _f1, _f2) = make_group().await;
        let state = g.get_state().await.unwrap();
        assert!(state.on);
        assert_eq!(state.brightness, 200);
    }

    #[tokio::test]
    async fn test_group_set_power() {
        let (g, ls, _f1, _f2) = make_group().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"on": false})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&ls)
            .await;

        g.set_power(false).await.unwrap();
    }

    #[tokio::test]
    async fn test_group_set_brightness() {
        let (g, ls, _f1, _f2) = make_group().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"bri": 50})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&ls)
            .await;

        g.set_brightness(50).await.unwrap();
    }

    #[tokio::test]
    async fn test_group_activate_preset() {
        let (g, ls, _f1, _f2) = make_group().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({"ps": 3})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&ls)
            .await;

        g.activate_preset("Red Party").await.unwrap();
    }

    #[tokio::test]
    async fn test_group_set_color() {
        let (g, ls, _f1, _f2) = make_group().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({
                "seg": [{"id": 0, "col": [[0, 255, 0], [0, 0, 0], [0, 0, 0]]}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&ls)
            .await;

        g.set_color(0, 255, 0).await.unwrap();
    }

    #[tokio::test]
    async fn test_group_set_effect() {
        let (g, ls, _f1, _f2) = make_group().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({
                "seg": [{"id": 0, "fx": 1}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&ls)
            .await;

        g.set_effect("Blink").await.unwrap();
    }

    #[tokio::test]
    async fn test_group_set_unknown_effect() {
        let (g, _ls, _f1, _f2) = make_group().await;
        let result = g.set_effect("Doesnt Exist").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_group_set_channel_color() {
        let (g, ls, _f1, _f2) = make_group().await;
        Mock::given(method("POST"))
            .and(path("/json/state"))
            .and(body_json(serde_json::json!({
                "seg": [{"id": 0, "col": [[0, 0, 255], [0, 0, 0], [0, 0, 0]]}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&ls)
            .await;

        g.set_channel_color(1, 0, 0, 255).await.unwrap();
    }

    #[tokio::test]
    async fn test_group_channel_zero_invalid() {
        let (g, _ls, _f1, _f2) = make_group().await;
        let result = g.set_channel_color(0, 255, 0, 0).await;
        assert!(matches!(result, Err(WledError::InvalidChannel { .. })));
    }

    #[tokio::test]
    async fn test_check_sync_health_detects_drift() {
        let (g, _ls, _f1, _f2) = make_group().await;
        let report = g.check_sync_health().await.unwrap();
        // follower-1 is in sync (ps=3), follower-2 is drifted (ps=99)
        assert!(!report.healthy);
        let f1 = report
            .devices
            .iter()
            .find(|d| d.device_name == "follower-1")
            .unwrap();
        let f2 = report
            .devices
            .iter()
            .find(|d| d.device_name == "follower-2")
            .unwrap();
        assert!(f1.is_healthy);
        assert!(!f2.is_healthy);
        assert_eq!(f2.device_preset, 99);
    }

    #[tokio::test]
    async fn test_check_sync_health_all_healthy() {
        let ls = MockServer::start().await;
        let f1 = MockServer::start().await;
        mount_leader_stubs(&ls).await;
        mount_follower_stubs(&f1, 3).await;

        let mut g = WledSyncGroup::new("g", "l", client_for(&ls, "l"), "DigUno");
        g.add_follower("f1", client_for(&f1, "f1"), "DigUno");
        let report = g.check_sync_health().await.unwrap();
        assert!(report.healthy);
    }

    #[tokio::test]
    async fn test_force_resync() {
        let (g, _ls, f1, f2) = make_group().await;
        // force_resync activates preset 3 on all followers.
        for srv in [&f1, &f2] {
            Mock::given(method("POST"))
                .and(path("/json/state"))
                .and(body_json(serde_json::json!({"ps": 3})))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
                .mount(srv)
                .await;
        }

        g.force_resync().await.unwrap();
    }

    #[tokio::test]
    async fn test_device_access() {
        let (g, _ls, _f1, _f2) = make_group().await;
        assert!(g.get_device("leader-1").is_some());
        assert!(g.get_device("follower-1").is_some());
        assert!(g.get_device("nope").is_none());
        assert_eq!(g.device_count(), 3);
    }
}
