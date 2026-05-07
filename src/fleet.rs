use std::collections::HashMap;
use std::path::Path;

use crate::client::WledClient;
use crate::config::FleetConfig;
use crate::error::WledError;
use crate::sync_group::WledSyncGroup;
use crate::types::*;

/// Manages a collection of sync groups loaded from a config file.
///
/// `WledFleet` is the top-level type passed to MCP tool handlers.
/// Devices are organized into named sync groups; all control operations are
/// group-scoped. Individual device access is available for troubleshooting.
#[derive(Clone)]
pub struct WledFleet {
    /// Sync groups, keyed by group name. Order matches the config file.
    groups: indexmap::IndexMap<String, WledSyncGroup>,
    /// Maps each device name to its group name for O(1) lookups.
    device_to_group: HashMap<String, String>,
}

impl WledFleet {
    /// Load a fleet from a `wled-config.toml` file.
    pub fn load_from_config(path: &Path) -> Result<Self, WledError> {
        let config = FleetConfig::load_from_file(path)?;
        Self::from_config(config)
    }

    /// Build a fleet from an already-parsed `FleetConfig`.
    pub fn from_config(config: FleetConfig) -> Result<Self, WledError> {
        config.validate()?;

        let mut groups = indexmap::IndexMap::new();
        let mut device_to_group: HashMap<String, String> = HashMap::new();

        for group_cfg in config.sync_groups {
            // Split into leader + followers based on is_leader flag.
            let (leader_cfgs, follower_cfgs): (Vec<_>, Vec<_>) =
                group_cfg.devices.into_iter().partition(|d| d.is_leader);

            // validate() guarantees exactly one leader.
            let leader_cfg = leader_cfgs.into_iter().next().unwrap();

            let leader_client = WledClient::builder(&leader_cfg.address)
                .device_name(&leader_cfg.name)
                .build()?;

            let mut sync_group = WledSyncGroup::new(
                &group_cfg.name,
                &leader_cfg.name,
                leader_client,
                &leader_cfg.device_type,
            );

            device_to_group.insert(leader_cfg.name.clone(), group_cfg.name.clone());

            for follower_cfg in follower_cfgs {
                let client = WledClient::builder(&follower_cfg.address)
                    .device_name(&follower_cfg.name)
                    .build()?;
                sync_group.add_follower(&follower_cfg.name, client, &follower_cfg.device_type);
                device_to_group.insert(follower_cfg.name.clone(), group_cfg.name.clone());
            }

            groups.insert(group_cfg.name, sync_group);
        }

        Ok(Self {
            groups,
            device_to_group,
        })
    }

    // ── Group access ──────────────────────────────────────────────────────────

    /// Returns all group names in config-file order.
    pub fn list_groups(&self) -> Vec<&str> {
        self.groups.keys().map(String::as_str).collect()
    }

    /// Returns the sync group with the given name, if it exists.
    pub fn get_group(&self, name: &str) -> Option<&WledSyncGroup> {
        self.groups.get(name)
    }

    /// Returns the sync group that contains the named device (leader or follower).
    pub fn get_group_for_device(&self, device_name: &str) -> Option<&WledSyncGroup> {
        let group_name = self.device_to_group.get(device_name)?;
        self.groups.get(group_name)
    }

    // ── Device access ─────────────────────────────────────────────────────────

    /// Returns the client for any device in the fleet by name.
    pub fn get_device(&self, device_name: &str) -> Option<&WledClient> {
        let group = self.get_group_for_device(device_name)?;
        group.get_device(device_name)
    }

    /// Returns all devices across all groups as `(device_name, group_name, client)`.
    pub fn list_all_devices(&self) -> Vec<(&str, &str, &WledClient)> {
        self.groups
            .iter()
            .flat_map(|(group_name, group)| {
                group
                    .list_devices()
                    .into_iter()
                    .map(move |(dev_name, client)| (dev_name, group_name.as_str(), client))
            })
            .collect()
    }

    // ── Fleet operations ──────────────────────────────────────────────────────

    /// Activates a preset by name on each of the named groups concurrently.
    #[tracing::instrument(skip(self), fields(preset = %preset_name, groups = ?group_names))]
    ///
    /// If any group fails, the others still proceed. All failures are
    /// combined into a single error message.
    pub async fn activate_preset_broadcast(
        &self,
        group_names: &[&str],
        preset_name: &str,
    ) -> Result<(), WledError> {
        let mut handles = tokio::task::JoinSet::new();

        for &name in group_names {
            match self.groups.get(name) {
                None => {
                    return Err(WledError::DeviceNotFound(name.to_string()));
                }
                Some(group) => {
                    let group = group.clone();
                    let preset = preset_name.to_string();
                    let gname = name.to_string();
                    handles.spawn(async move {
                        (gname, group.activate_preset(&preset).await)
                    });
                }
            }
        }

        let mut errors: Vec<String> = Vec::new();
        while let Some(res) = handles.join_next().await {
            let (name, result) = res.expect("task panicked");
            if let Err(e) = result {
                errors.push(format!("{}: {}", name, e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(WledError::ConfigError(format!(
                "Broadcast failed for {} group(s): {}",
                errors.len(),
                errors.join(", ")
            )))
        }
    }

    #[tracing::instrument(skip(self))]
    /// Returns a status snapshot for every group in the fleet.
    ///
    /// Groups that fail to respond are included with default/zero values rather
    /// than causing the entire call to fail.
    pub async fn get_fleet_status(&self) -> FleetStatus {
        let mut handles = tokio::task::JoinSet::new();

        for (group_name, group) in &self.groups {
            let group = group.clone();
            let gname = group_name.clone();
            let leader_name = group.leader_name().to_string();
            let device_count = group.device_count();
            handles.spawn(async move {
                let state_result = group.get_state().await;
                (gname, leader_name, device_count, state_result)
            });
        }

        let mut statuses: Vec<GroupStatus> = Vec::new();
        while let Some(res) = handles.join_next().await {
            let (group_name, leader_name, device_count, state_result) =
                res.expect("task panicked");
            let (on, brightness, active_preset) = match state_result {
                Ok(s) => (s.on, s.brightness, s.preset_slot),
                Err(e) => {
                    tracing::warn!(group = %group_name, error = %e, "Failed to get group status");
                    (false, 0, -1)
                }
            };
            statuses.push(GroupStatus {
                group_name,
                leader_name,
                on,
                brightness,
                active_preset,
                device_count,
            });
        }

        // Re-sort into config-file order.
        let order: HashMap<&str, usize> = self
            .groups
            .keys()
            .enumerate()
            .map(|(i, k)| (k.as_str(), i))
            .collect();
        statuses.sort_by_key(|s| order.get(s.group_name.as_str()).copied().unwrap_or(usize::MAX));

        let total_devices = statuses.iter().map(|s| s.device_count).sum();
        let total_groups = statuses.len();

        FleetStatus {
            groups: statuses,
            total_groups,
            total_devices,
        }
    }

    #[tracing::instrument(skip(self))]
    /// Checks sync health for every group concurrently.
    ///
    /// Returns a map of group name → health report. Groups that fail to
    /// respond are omitted from the map.
    pub async fn check_all_sync_health(&self) -> HashMap<String, SyncHealthReport> {
        let mut handles = tokio::task::JoinSet::new();

        for (group_name, group) in &self.groups {
            let group = group.clone();
            let gname = group_name.clone();
            handles.spawn(async move { (gname, group.check_sync_health().await) });
        }

        let mut reports = HashMap::new();
        while let Some(res) = handles.join_next().await {
            let (name, result) = res.expect("task panicked");
            match result {
                Ok(report) => {
                    reports.insert(name, report);
                }
                Err(e) => {
                    tracing::warn!(group = %name, error = %e, "Sync health check failed");
                }
            }
        }
        reports
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DeviceConfig, FleetConfig, SyncGroupConfig};

    fn make_config() -> FleetConfig {
        FleetConfig {
            sync_groups: vec![
                SyncGroupConfig {
                    name: "lounge".to_string(),
                    leader: Some("wled-1".to_string()),
                    devices: vec![
                        DeviceConfig {
                            name: "wled-1".to_string(),
                            address: "192.168.1.1".to_string(),
                            is_leader: true,
                            device_type: "DigQuad".to_string(),
                        },
                        DeviceConfig {
                            name: "wled-2".to_string(),
                            address: "192.168.1.2".to_string(),
                            is_leader: false,
                            device_type: "DigUno".to_string(),
                        },
                    ],
                },
                SyncGroupConfig {
                    name: "patio".to_string(),
                    leader: None,
                    devices: vec![DeviceConfig {
                        name: "wled-3".to_string(),
                        address: "192.168.1.3".to_string(),
                        is_leader: true,
                        device_type: "Dig2Go".to_string(),
                    }],
                },
            ],
            schedule: None,
        }
    }

    fn make_fleet() -> WledFleet {
        WledFleet::from_config(make_config()).unwrap()
    }

    #[test]
    fn test_list_groups() {
        let fleet = make_fleet();
        let groups = fleet.list_groups();
        assert_eq!(groups, vec!["lounge", "patio"]);
    }

    #[test]
    fn test_get_group() {
        let fleet = make_fleet();
        assert!(fleet.get_group("lounge").is_some());
        assert!(fleet.get_group("nope").is_none());
    }

    #[test]
    fn test_get_device() {
        let fleet = make_fleet();
        assert!(fleet.get_device("wled-1").is_some());
        assert!(fleet.get_device("wled-2").is_some());
        assert!(fleet.get_device("wled-3").is_some());
        assert!(fleet.get_device("wled-99").is_none());
    }

    #[test]
    fn test_get_group_for_device() {
        let fleet = make_fleet();
        let group = fleet.get_group_for_device("wled-2").unwrap();
        assert_eq!(group.name, "lounge");
        let group = fleet.get_group_for_device("wled-3").unwrap();
        assert_eq!(group.name, "patio");
    }

    #[test]
    fn test_list_all_devices() {
        let fleet = make_fleet();
        let devices = fleet.list_all_devices();
        assert_eq!(devices.len(), 3);
    }

    #[test]
    fn test_config_validation_no_leader() {
        let mut config = make_config();
        config.sync_groups[0].devices[0].is_leader = false;
        let result = WledFleet::from_config(config);
        assert!(matches!(result, Err(WledError::ConfigError(_))));
    }

    #[test]
    fn test_config_validation_two_leaders() {
        let mut config = make_config();
        config.sync_groups[0].devices[1].is_leader = true;
        let result = WledFleet::from_config(config);
        assert!(matches!(result, Err(WledError::ConfigError(_))));
    }

    #[tokio::test]
    async fn test_fleet_status_order() {
        let fleet = make_fleet();
        let status = fleet.get_fleet_status().await;
        // Groups should appear in config order even though they're fetched concurrently.
        assert_eq!(status.groups[0].group_name, "lounge");
        assert_eq!(status.groups[1].group_name, "patio");
        assert_eq!(status.total_groups, 2);
        assert_eq!(status.total_devices, 3);
    }
}
