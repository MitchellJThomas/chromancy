use rmcp::{ServerHandler, model::{ServerCapabilities, ServerInfo}, schemars, tool};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::fleet::WledFleet;

// ── Parameter types ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GroupNameParam {
    #[schemars(description = "Name of the sync group")]
    pub group_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OptionalGroupNameParam {
    #[schemars(description = "Name of the sync group (omit for all groups)")]
    pub group_name: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeviceNameParam {
    #[schemars(description = "Name of the device")]
    pub device_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ActivatePresetParam {
    #[schemars(description = "Name of the sync group")]
    pub group_name: String,
    #[schemars(description = "Name of the preset to activate")]
    pub preset_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BroadcastPresetParam {
    #[schemars(description = "Names of sync groups to target")]
    pub group_names: Vec<String>,
    #[schemars(description = "Name of the preset to activate")]
    pub preset_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetPowerParam {
    #[schemars(description = "Name of the sync group")]
    pub group_name: String,
    #[schemars(description = "true to turn on, false to turn off")]
    pub on: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetBrightnessParam {
    #[schemars(description = "Name of the sync group")]
    pub group_name: String,
    #[schemars(description = "Brightness value 0-255")]
    pub brightness: u8,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetColorParam {
    #[schemars(description = "Name of the sync group")]
    pub group_name: String,
    #[schemars(description = "Red channel 0-255")]
    pub r: u8,
    #[schemars(description = "Green channel 0-255")]
    pub g: u8,
    #[schemars(description = "Blue channel 0-255")]
    pub b: u8,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetEffectParam {
    #[schemars(description = "Name of the sync group")]
    pub group_name: String,
    #[schemars(description = "Name of the effect to activate")]
    pub effect_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SetPaletteParam {
    #[schemars(description = "Name of the sync group")]
    pub group_name: String,
    #[schemars(description = "Name of the palette to activate")]
    pub palette_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IndividualPowerParam {
    #[schemars(description = "Name of the device")]
    pub device_name: String,
    #[schemars(description = "true to turn on, false to turn off")]
    pub on: bool,
}

// ── Server ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ChromancyServer {
    pub fleet: WledFleet,
}

#[tool(tool_box)]
impl ChromancyServer {
    // ── Group Management ───────────────────────────────────────────────────────

    #[tool(description = "List all sync group names in the fleet")]
    fn list_groups(&self) -> String {
        let groups = self.fleet.list_groups();
        serde_json::to_string(&groups).unwrap_or_else(|_| "[]".to_string())
    }

    #[tool(description = "List devices in a group, or all devices if no group specified")]
    fn list_devices(&self, #[tool(aggr)] p: OptionalGroupNameParam) -> String {
        match p.group_name.as_deref() {
            Some(gname) => match self.fleet.get_group(gname) {
                None => json!({"error": format!("Group '{}' not found", gname)}).to_string(),
                Some(group) => {
                    let devs: Vec<String> = group
                        .list_devices()
                        .into_iter()
                        .map(|(n, _)| n.to_string())
                        .collect();
                    serde_json::to_string(&devs).unwrap_or_else(|_| "[]".to_string())
                }
            },
            None => {
                let all: Vec<serde_json::Value> = self
                    .fleet
                    .list_all_devices()
                    .into_iter()
                    .map(|(dev, group, _)| json!({"device": dev, "group": group}))
                    .collect();
                serde_json::to_string(&all).unwrap_or_else(|_| "[]".to_string())
            }
        }
    }

    // ── Device Queries ─────────────────────────────────────────────────────────

    #[tool(description = "Get device capabilities (LED count, firmware version, uptime)")]
    async fn get_device_info(&self, #[tool(aggr)] p: DeviceNameParam) -> String {
        match self.fleet.get_device(&p.device_name).cloned() {
            None => json!({"error": format!("Device '{}' not found", p.device_name)}).to_string(),
            Some(client) => match client.get_info().await {
                Ok(info) => serde_json::to_string(&info).unwrap_or_else(|_| "{}".to_string()),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Get current state of a device (power, brightness, effect, palette)")]
    async fn get_device_state(&self, #[tool(aggr)] p: DeviceNameParam) -> String {
        match self.fleet.get_device(&p.device_name).cloned() {
            None => json!({"error": format!("Device '{}' not found", p.device_name)}).to_string(),
            Some(client) => match client.get_state().await {
                Ok(state) => serde_json::to_string(&state).unwrap_or_else(|_| "{}".to_string()),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Get status of a specific sync group")]
    async fn get_group_status(&self, #[tool(aggr)] p: GroupNameParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.get_state().await {
                Ok(state) => json!({
                    "group": p.group_name,
                    "on": state.on,
                    "brightness": state.brightness,
                    "active_preset": state.preset_slot,
                })
                .to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Get status of the entire fleet (all groups)")]
    async fn get_fleet_status(&self) -> String {
        let status = self.fleet.get_fleet_status().await;
        serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string())
    }

    // ── Group Control ──────────────────────────────────────────────────────────

    #[tool(description = "Activate a preset by name on a sync group (leader syncs to followers)")]
    async fn activate_preset(&self, #[tool(aggr)] p: ActivatePresetParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.activate_preset(&p.preset_name).await {
                Ok(()) => json!({"success": true}).to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Activate the same preset across multiple sync groups concurrently")]
    async fn activate_preset_broadcast(&self, #[tool(aggr)] p: BroadcastPresetParam) -> String {
        let group_refs: Vec<&str> = p.group_names.iter().map(String::as_str).collect();
        match self
            .fleet
            .activate_preset_broadcast(&group_refs, &p.preset_name)
            .await
        {
            Ok(()) => json!({"success": true}).to_string(),
            Err(e) => json!({"error": e.to_string()}).to_string(),
        }
    }

    #[tool(description = "List available presets on a group's leader device")]
    async fn list_presets(&self, #[tool(aggr)] p: GroupNameParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.list_presets().await {
                Ok(presets) => {
                    let list: Vec<serde_json::Value> = presets
                        .iter()
                        .map(|(id, preset)| json!({"id": id, "name": preset.name}))
                        .collect();
                    serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string())
                }
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Turn a sync group on or off")]
    async fn set_power(&self, #[tool(aggr)] p: SetPowerParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.set_power(p.on).await {
                Ok(()) => json!({"success": true, "power": p.on}).to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Set master brightness (0-255) on a sync group")]
    async fn set_brightness(&self, #[tool(aggr)] p: SetBrightnessParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.set_brightness(p.brightness).await {
                Ok(()) => json!({"success": true, "brightness": p.brightness}).to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Set primary RGB color on a sync group's leader (segment 0)")]
    async fn set_color(&self, #[tool(aggr)] p: SetColorParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.set_color(p.r, p.g, p.b).await {
                Ok(()) => json!({"success": true}).to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Set active effect by name on a sync group")]
    async fn set_effect(&self, #[tool(aggr)] p: SetEffectParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.set_effect(&p.effect_name).await {
                Ok(()) => json!({"success": true}).to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Set active palette by name on a sync group")]
    async fn set_palette(&self, #[tool(aggr)] p: SetPaletteParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.set_palette(&p.palette_name).await {
                Ok(()) => json!({"success": true}).to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    // ── Sync Health & Troubleshooting ──────────────────────────────────────────

    #[tool(description = "Check sync health — specific group or all groups if omitted")]
    async fn check_sync_health(&self, #[tool(aggr)] p: OptionalGroupNameParam) -> String {
        match p.group_name.as_deref() {
            Some(gname) => match self.fleet.get_group(gname).cloned() {
                None => json!({"error": format!("Group '{}' not found", gname)}).to_string(),
                Some(group) => match group.check_sync_health().await {
                    Ok(report) => {
                        serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string())
                    }
                    Err(e) => json!({"error": e.to_string()}).to_string(),
                },
            },
            None => {
                let reports = self.fleet.check_all_sync_health().await;
                serde_json::to_string(&reports).unwrap_or_else(|_| "{}".to_string())
            }
        }
    }

    #[tool(description = "Force all followers in a group to re-sync with the leader")]
    async fn force_resync(&self, #[tool(aggr)] p: GroupNameParam) -> String {
        match self.fleet.get_group(&p.group_name).cloned() {
            None => {
                json!({"error": format!("Group '{}' not found", p.group_name)}).to_string()
            }
            Some(group) => match group.force_resync().await {
                Ok(()) => json!({"success": true}).to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Get state of a single device independently (for troubleshooting)")]
    async fn get_individual_state(&self, #[tool(aggr)] p: DeviceNameParam) -> String {
        match self.fleet.get_device(&p.device_name).cloned() {
            None => json!({"error": format!("Device '{}' not found", p.device_name)}).to_string(),
            Some(client) => match client.get_state().await {
                Ok(state) => serde_json::to_string(&state).unwrap_or_else(|_| "{}".to_string()),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }

    #[tool(description = "Control a single device's power independently (bypasses group sync)")]
    async fn set_individual_power(&self, #[tool(aggr)] p: IndividualPowerParam) -> String {
        match self.fleet.get_device(&p.device_name).cloned() {
            None => json!({"error": format!("Device '{}' not found", p.device_name)}).to_string(),
            Some(client) => match client.set_power(p.on).await {
                Ok(()) => json!({"success": true, "power": p.on}).to_string(),
                Err(e) => json!({"error": e.to_string()}).to_string(),
            },
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for ChromancyServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Chromancy MCP server — controls WLED LED sync groups for lighting art"
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
