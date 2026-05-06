use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// RGB color [r, g, b]
pub type Color = [u8; 3];

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
    #[serde(default = "default_brightness")]
    pub bri: u8,
    /// Crossfade time in 100 ms units
    #[serde(default)]
    pub transition: u16,
    /// Active preset slot (−1 = none)
    #[serde(default = "default_neg_one_i32")]
    pub ps: i32,
    /// Active playlist slot (−1 = none)
    #[serde(default = "default_neg_one_i32")]
    pub pl: i32,
    /// Nightlight state
    #[serde(default)]
    pub nl: NightlightState,
    /// UDP sync settings
    #[serde(default)]
    pub udpn: UdpSync,
    /// Live data override (0 = off, 1 = override, 2 = full)
    #[serde(default)]
    pub lor: u8,
    /// Main segment index
    #[serde(default)]
    pub mainseg: u8,
    /// LED segments
    #[serde(default)]
    pub seg: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NightlightState {
    #[serde(default)]
    pub on: bool,
    #[serde(default = "default_60_u8")]
    pub dur: u8,
    /// Mode: 0 = instant, 1 = fade, 2 = color fade, 3 = sunrise
    #[serde(default)]
    pub mode: u8,
    /// Target brightness
    #[serde(default)]
    pub tbri: u8,
    /// Remaining time in seconds (−1 = inactive)
    #[serde(default = "default_neg_one_i16")]
    pub rem: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UdpSync {
    #[serde(default)]
    pub send: bool,
    #[serde(default = "default_true")]
    pub recv: bool,
    /// Send group bitmask
    #[serde(default = "default_1_u8")]
    pub sgrp: u8,
    /// Receive group bitmask
    #[serde(default = "default_1_u8")]
    pub rgrp: u8,
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
    #[serde(default = "default_1_u8")]
    pub grp: u8,
    #[serde(default)]
    pub spc: u8,
    #[serde(default)]
    pub of: u16,
    #[serde(default = "default_true")]
    pub on: bool,
    #[serde(default)]
    pub frz: bool,
    #[serde(default = "default_brightness")]
    pub bri: u8,
    /// Color temperature
    #[serde(default)]
    pub cct: u16,
    /// Colors: [[r,g,b], [r,g,b], [r,g,b]] (primary, secondary, tertiary)
    #[serde(default)]
    pub col: Vec<Color>,
    /// Effect ID
    #[serde(default)]
    pub fx: u16,
    /// Effect speed
    #[serde(default = "default_128_u8")]
    pub sx: u8,
    /// Effect intensity
    #[serde(default = "default_128_u8")]
    pub ix: u8,
    /// Palette ID
    #[serde(default)]
    pub pal: u16,
    #[serde(default = "default_128_u8")]
    pub c1: u8,
    #[serde(default = "default_128_u8")]
    pub c2: u8,
    #[serde(default)]
    pub c3: u8,
    #[serde(default = "default_true")]
    pub sel: bool,
    #[serde(default)]
    pub rev: bool,
    #[serde(default)]
    pub mi: bool,
}

/// Partial state update sent to `/json/state`.
/// Only set fields are serialized and applied on the device.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WledStateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bri: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<u16>,
    /// Activate preset by slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ps: Option<i32>,
    /// Save current state to preset slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub psave: Option<i32>,
    /// Delete preset slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdel: Option<i32>,
    /// Preset name (used with `psave`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<String>,
    /// Activate playlist by slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pl: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nl: Option<NightlightRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub udpn: Option<UdpSyncRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lor: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mainseg: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seg: Option<Vec<SegmentRequest>>,
    /// Return updated state in response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NightlightRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dur: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tbri: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UdpSyncRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub send: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recv: Option<bool>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bri: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<Vec<Color>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sx: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ix: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pal: Option<u16>,
}

/// Device information from `/json/info`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WledInfo {
    /// Firmware version string
    #[serde(default)]
    pub ver: String,
    /// Build ID
    #[serde(default)]
    pub vid: u32,
    #[serde(default)]
    pub leds: LedInfo,
    /// Device name (AP/mDNS name)
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub udpport: u16,
    #[serde(default)]
    pub live: bool,
    /// Number of effects
    #[serde(default)]
    pub fxcount: u16,
    /// Number of palettes
    #[serde(default)]
    pub palcount: u16,
    /// Platform (e.g. "esp32")
    #[serde(default)]
    pub arch: String,
    #[serde(default)]
    pub core: String,
    #[serde(default)]
    pub freeheap: u32,
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
    #[serde(default)]
    pub pwr: u32,
    /// Current FPS
    #[serde(default)]
    pub fps: u32,
    /// Max power budget in mA
    #[serde(default)]
    pub maxpwr: u32,
    /// Max segments
    #[serde(default)]
    pub maxseg: u8,
    #[serde(default)]
    pub rgbw: bool,
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
    #[serde(default)]
    pub n: String,
    /// Quick label (shown in WLED UI)
    #[serde(default)]
    pub ql: String,
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
