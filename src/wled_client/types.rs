// src/wled_client/types.rs
use serde::{Deserialize, Serialize};

/// Complete state response from /json/state
#[derive(Debug, Clone, Deserialize)]
pub struct WledState {
    pub on: bool,
    pub bri: u8,
    pub transition: u16,
    pub ps: i8,              // playlist ID (-1 = none)
    pub pl: i8,              // playlist position
    pub nl: Nightlight,
    pub udpn: UdpNetwork,
    pub seg: Vec<Segment>,
}

/// State request (partial, for updates)
#[derive(Debug, Clone, Serialize, Default)]
pub struct WledStateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bri: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seg: Option<Vec<SegmentRequest>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Nightlight {
    pub on: bool,
    pub dur: u16,
    pub fade: bool,
    pub tbri: u8,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct UdpNetwork {
    pub send: bool,
    pub recv: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Segment {
    pub start: usize,
    pub stop: usize,
    pub len: usize,
    pub col: Vec<[u8; 4]>,     // RGBA colors
    pub fx: usize,              // effect ID
    pub sx: u8,                 // effect speed
    pub ix: u8,                 // effect intensity
    pub pal: usize,             // palette ID
    pub sel: bool,              // selected
    pub rev: bool,              // reversed
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SegmentRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col: Option<Vec<[u8; 4]>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fx: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pal: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sx: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ix: Option<u8>,
}

/// Device info from /json/info
#[derive(Debug, Clone, Deserialize)]
pub struct WledInfo {
    pub ver: String,
    pub vid: u64,
    pub leds: LedInfo,
    pub name: String,
    pub udpport: u16,
    pub live: bool,
    pub fxcount: usize,
    pub palcount: usize,
    pub arch: String,
    pub uptime: u64,
    pub mac: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LedInfo {
    pub count: usize,
    pub rgbw: bool,
    pub pin: Vec<u8>,
    pub pwr: u32,
    pub maxpwr: u32,
    pub maxseg: u8,
}

/// Complete response from /json (state + info + effects + palettes)
#[derive(Debug, Clone, Deserialize)]
pub struct WledFullState {
    pub state: WledState,
    pub info: WledInfo,
    pub effects: Vec<String>,
    pub palettes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Effect {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Palette {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaletteColors {
    // Detailed color data for a palette
    // Structure varies by WLED version
    #[serde(flatten)]
    pub data: serde_json::Value,
}