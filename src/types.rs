use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RGB or RGBW color vector ([r, g, b] or [r, g, b, w]).
pub type Color = Vec<u8>;

fn default_brightness() -> u8 {
    128
}
fn default_neg_one_i32() -> i32 {
    -1
}
fn default_neg_one_i16() -> i16 {
    -1
}
fn default_true() -> bool {
    true
}
fn default_1_u8() -> u8 {
    1
}
fn default_60_u8() -> u8 {
    60
}
fn default_128_u8() -> u8 {
    128
}

/// Current state of a WLED device (mirrors WLED `/json/state` response).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WledState {
    /// Power on/off
    #[serde(default)]
    pub on: bool,
    /// Master brightness (0–255)
    #[serde(rename = "bri", default = "default_brightness")]
    pub brightness: u8,
    /// Crossfade time in 100 ms units
    #[serde(default)]
    pub transition: u16,
    /// Active preset slot (−1 = none)
    #[serde(rename = "ps", default = "default_neg_one_i32")]
    pub preset_slot: i32,
    /// Active playlist slot (−1 = none)
    #[serde(rename = "pl", default = "default_neg_one_i32")]
    pub playlist_slot: i32,
    /// Nightlight state
    #[serde(rename = "nl", default)]
    pub nightlight: NightlightState,
    /// UDP sync settings
    #[serde(rename = "udpn", default)]
    pub udp_sync: UdpSync,
    /// Live data override (0 = off, 1 = override, 2 = full)
    #[serde(rename = "lor", default)]
    pub live_override: u8,
    /// Main segment index
    #[serde(rename = "mainseg", default)]
    pub main_segment: u8,
    /// LED segments
    #[serde(rename = "seg", default)]
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NightlightState {
    #[serde(default)]
    pub on: bool,
    /// Duration in minutes
    #[serde(rename = "dur", default = "default_60_u8")]
    pub duration_minutes: u8,
    /// Mode: 0 = instant, 1 = fade, 2 = color fade, 3 = sunrise
    #[serde(default)]
    pub mode: u8,
    /// Target brightness
    #[serde(rename = "tbri", default)]
    pub target_brightness: u8,
    /// Remaining time in seconds (−1 = inactive)
    #[serde(rename = "rem", default = "default_neg_one_i16")]
    pub remaining_seconds: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UdpSync {
    #[serde(default)]
    pub send: bool,
    #[serde(default = "default_true")]
    pub recv: bool,
    /// Send group bitmask
    #[serde(rename = "sgrp", default = "default_1_u8")]
    pub send_group_mask: u8,
    /// Receive group bitmask
    #[serde(rename = "rgrp", default = "default_1_u8")]
    pub receive_group_mask: u8,
}

/// A single LED segment.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Segment {
    #[serde(default)]
    pub id: u8,
    #[serde(default)]
    pub start: u16,
    /// End index (exclusive)
    #[serde(default)]
    pub stop: u16,
    #[serde(default)]
    pub len: u16,
    /// Group size (how many LEDs are grouped together)
    #[serde(rename = "grp", default = "default_1_u8")]
    pub group_size: u8,
    /// Spacing between grouped LEDs
    #[serde(rename = "spc", default)]
    pub spacing: u8,
    /// Offset into the LED strip
    #[serde(rename = "of", default)]
    pub offset: u16,
    #[serde(default = "default_true")]
    pub on: bool,
    /// Freeze the segment (no animation updates)
    #[serde(rename = "frz", default)]
    pub frozen: bool,
    #[serde(rename = "bri", default = "default_brightness")]
    pub brightness: u8,
    /// Color temperature
    #[serde(rename = "cct", default)]
    pub color_temperature: u16,
    /// Colors: [[r,g,b], [r,g,b], [r,g,b]] (primary, secondary, tertiary)
    #[serde(rename = "col", default)]
    pub colors: Vec<Color>,
    /// Effect ID
    #[serde(rename = "fx", default)]
    pub effect_id: u16,
    /// Effect speed
    #[serde(rename = "sx", default = "default_128_u8")]
    pub effect_speed: u8,
    /// Effect intensity
    #[serde(rename = "ix", default = "default_128_u8")]
    pub effect_intensity: u8,
    /// Palette ID
    #[serde(rename = "pal", default)]
    pub palette_id: u16,
    #[serde(rename = "c1", default = "default_128_u8")]
    pub custom1: u8,
    #[serde(rename = "c2", default = "default_128_u8")]
    pub custom2: u8,
    #[serde(rename = "c3", default)]
    pub custom3: u8,
    /// Whether this segment is selected in the WLED UI
    #[serde(rename = "sel", default = "default_true")]
    pub selected: bool,
    /// Reverse the segment direction
    #[serde(rename = "rev", default)]
    pub reversed: bool,
    /// Mirror the segment
    #[serde(rename = "mi", default)]
    pub mirror: bool,
}

/// Partial state update sent to `/json/state`.
/// Only set fields are serialized and applied on the device.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WledStateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(rename = "bri", skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<u16>,
    /// Activate preset by slot
    #[serde(rename = "ps", skip_serializing_if = "Option::is_none")]
    pub preset_slot: Option<i32>,
    /// Save current state to preset slot
    #[serde(rename = "psave", skip_serializing_if = "Option::is_none")]
    pub preset_save_slot: Option<i32>,
    /// Delete preset slot
    #[serde(rename = "pdel", skip_serializing_if = "Option::is_none")]
    pub preset_delete_slot: Option<i32>,
    /// Preset name (used with `preset_save_slot`)
    #[serde(rename = "n", skip_serializing_if = "Option::is_none")]
    pub preset_name: Option<String>,
    /// Activate playlist by slot
    #[serde(rename = "pl", skip_serializing_if = "Option::is_none")]
    pub playlist_slot: Option<i32>,
    #[serde(rename = "lor", skip_serializing_if = "Option::is_none")]
    pub live_override: Option<u8>,
    #[serde(rename = "mainseg", skip_serializing_if = "Option::is_none")]
    pub main_segment: Option<u8>,
    #[serde(rename = "seg", skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<SegmentRequest>>,
    /// Return updated state in response
    #[serde(rename = "v", skip_serializing_if = "Option::is_none")]
    pub return_state: Option<bool>,
}

/// Partial segment update.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SegmentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(rename = "bri", skip_serializing_if = "Option::is_none")]
    pub brightness: Option<u8>,
    #[serde(rename = "col", skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<Color>>,
    #[serde(rename = "fx", skip_serializing_if = "Option::is_none")]
    pub effect_id: Option<u16>,
    #[serde(rename = "sx", skip_serializing_if = "Option::is_none")]
    pub effect_speed: Option<u8>,
    #[serde(rename = "ix", skip_serializing_if = "Option::is_none")]
    pub effect_intensity: Option<u8>,
    #[serde(rename = "pal", skip_serializing_if = "Option::is_none")]
    pub palette_id: Option<u16>,
}

/// Device information from `/json/info`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WledInfo {
    /// Firmware version string
    #[serde(rename = "ver", default)]
    pub firmware_version: String,
    /// Build ID
    #[serde(rename = "vid", default)]
    pub build_id: u32,
    #[serde(rename = "leds", default)]
    pub led_info: LedInfo,
    /// Device name (AP/mDNS name)
    #[serde(default)]
    pub name: String,
    #[serde(rename = "udpport", default)]
    pub udp_port: u16,
    #[serde(default)]
    pub live: bool,
    /// Number of effects
    #[serde(rename = "fxcount", default)]
    pub effect_count: u16,
    /// Number of palettes
    #[serde(rename = "palcount", default)]
    pub palette_count: u16,
    /// Platform (e.g. "esp32")
    #[serde(rename = "arch", default)]
    pub platform: String,
    #[serde(default)]
    pub core: String,
    #[serde(rename = "freeheap", default)]
    pub free_heap: u32,
    /// Uptime in seconds
    #[serde(default)]
    pub uptime: u64,
    #[serde(default)]
    pub brand: String,
    #[serde(default)]
    pub product: String,
    #[serde(default)]
    pub mac: String,
    #[serde(default)]
    pub ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LedInfo {
    /// Total LED count
    #[serde(default)]
    pub count: u32,
    /// Current power draw in mA
    #[serde(rename = "pwr", default)]
    pub power_draw_ma: u32,
    /// Current FPS
    #[serde(default)]
    pub fps: u32,
    /// Max power budget in mA
    #[serde(rename = "maxpwr", default)]
    pub max_power_ma: u32,
    /// Max segments
    #[serde(rename = "maxseg", default)]
    pub max_segments: u8,
    /// Whether the device has a dedicated white channel (RGBW)
    #[serde(rename = "rgbw", default)]
    pub has_white_channel: bool,
}

/// Full state response from `/json`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WledFullState {
    pub state: WledState,
    pub info: WledInfo,
    #[serde(default)]
    pub effects: Vec<String>,
    #[serde(default)]
    pub palettes: Vec<String>,
}

/// Preset entry from `/json/presets`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PresetInfo {
    /// Preset name
    #[serde(rename = "n", default)]
    pub name: String,
    /// Quick label (shown in WLED UI)
    #[serde(rename = "ql", default)]
    pub quick_label: String,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// NTP server address — either a DNS hostname or a dotted-decimal IPv4/IPv6
/// address string.
///
/// ```rust
/// # use chromancy::NtpServer;
/// let ntp = NtpServer::hostname("pool.ntp.org");
/// let ntp = NtpServer::ip([216, 239, 35, 0]);   // time.google.com
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NtpServer {
    /// A DNS hostname, e.g. `"pool.ntp.org"` or `"time.cloudflare.com"`.
    Hostname(String),
    /// A raw IPv4 address.
    Ipv4([u8; 4]),
}

impl NtpServer {
    pub fn hostname(host: impl Into<String>) -> Self {
        Self::Hostname(host.into())
    }

    pub fn ip(octets: [u8; 4]) -> Self {
        Self::Ipv4(octets)
    }

    /// Returns the server as the string WLED expects in its config JSON.
    pub fn as_str(&self) -> String {
        match self {
            Self::Hostname(h) => h.clone(),
            Self::Ipv4([a, b, c, d]) => format!("{}.{}.{}.{}", a, b, c, d),
        }
    }
}

impl Default for NtpServer {
    fn default() -> Self {
        Self::Hostname("pool.ntp.org".to_string())
    }
}

impl std::fmt::Display for NtpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.as_str())
    }
}

/// NTP configuration.
#[derive(Debug, Clone, Default)]
pub struct NtpConfig {
    pub server: NtpServer,
}

/// Dusk-based schedule configuration.
#[derive(Debug, Clone)]
pub struct DuskScheduleConfig {
    /// Preset slot to activate at dusk
    pub preset_slot: i32,
    /// Hour to turn off (0–23)
    pub off_hour: u8,
    /// Minute to turn off (0–59)
    pub off_minute: u8,
    pub enabled: bool,
}

// ── Sync health types ─────────────────────────────────────────────────────────

/// Per-device sync status within a group.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceSyncStatus {
    pub device_name: String,
    /// True if the device's active preset matches the leader's.
    pub is_healthy: bool,
    pub leader_preset: i32,
    pub device_preset: i32,
}

/// Sync health report for a single sync group.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncHealthReport {
    pub group_name: String,
    /// True only if all follower devices are in sync with the leader.
    pub healthy: bool,
    pub devices: Vec<DeviceSyncStatus>,
}

// ── Fleet status types ────────────────────────────────────────────────────────

/// State summary for one sync group.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GroupStatus {
    pub group_name: String,
    pub leader_name: String,
    pub on: bool,
    pub brightness: u8,
    pub active_preset: i32,
    /// Total devices in the group (leader + followers).
    pub device_count: usize,
}

/// Fleet-wide status snapshot.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FleetStatus {
    pub groups: Vec<GroupStatus>,
    pub total_groups: usize,
    pub total_devices: usize,
}
