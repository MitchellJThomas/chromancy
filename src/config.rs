use serde::Deserialize;
use std::path::Path;

use crate::error::WledError;

/// Top-level fleet configuration (parsed from `wled-config.toml`).
#[derive(Debug, Deserialize, Default)]
pub struct FleetConfig {
    #[serde(default)]
    pub sync_groups: Vec<SyncGroupConfig>,
    pub schedule: Option<ScheduleConfig>,
}

/// Configuration for one sync group.
#[derive(Debug, Deserialize)]
pub struct SyncGroupConfig {
    pub name: String,
    /// Optional documentation hint; actual leader is determined by `is_leader = true` in devices.
    pub leader: Option<String>,
    #[serde(default)]
    pub devices: Vec<DeviceConfig>,
}

/// Configuration for a single WLED device.
#[derive(Debug, Deserialize)]
pub struct DeviceConfig {
    pub name: String,
    pub address: String,
    #[serde(default)]
    pub is_leader: bool,
    #[serde(default)]
    pub device_type: String,
}

/// Global schedule configuration.
#[derive(Debug, Deserialize, Default)]
pub struct ScheduleConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Preset name to activate at dusk.
    #[serde(default)]
    pub dusk_preset: String,
    /// Time to turn off (format: "HH:MM").
    #[serde(default)]
    pub off_time: String,
}

impl FleetConfig {
    /// Load fleet configuration from a TOML file.
    pub fn load_from_file(path: &Path) -> Result<Self, WledError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            WledError::ConfigError(format!("Cannot read {}: {}", path.display(), e))
        })?;
        toml::from_str(&content).map_err(|e| {
            WledError::ConfigError(format!("Parse error in {}: {}", path.display(), e))
        })
    }

    /// Validate the config. Returns an error if any sync group has no leader or
    /// duplicate leaders.
    pub fn validate(&self) -> Result<(), WledError> {
        for group in &self.sync_groups {
            if group.devices.is_empty() {
                return Err(WledError::ConfigError(format!(
                    "Sync group '{}' has no devices",
                    group.name
                )));
            }
            let leader_count = group.devices.iter().filter(|d| d.is_leader).count();
            match leader_count {
                0 => {
                    return Err(WledError::ConfigError(format!(
                        "Sync group '{}' has no leader (set is_leader = true on one device)",
                        group.name
                    )))
                }
                1 => {}
                n => {
                    return Err(WledError::ConfigError(format!(
                        "Sync group '{}' has {} leaders (only one allowed)",
                        group.name, n
                    )))
                }
            }
        }
        Ok(())
    }
}
