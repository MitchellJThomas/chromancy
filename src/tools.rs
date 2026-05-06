// src/tools.rs
use rmcp::{tool, ToolHandler};
use crate::wled_client::WledClient;

#[tool(description = "Set the brightness of a WLED device")]
pub struct SetBrightness;

#[async_trait::async_trait]
impl ToolHandler for SetBrightness {
    type Parameters = SetBrightnessParams;
    type Result = serde_json::Value;

    async fn call(&self, params: Self::Parameters) -> Result<Self::Result, McpError> {
        let client = WledClient::new(&params.device_id)?;
        
        let state = WledState {
            bri: Some(params.brightness),
            ..Default::default()
        };
        
        client.set_state(&state).await
            .map_err(|e| McpError::internal_error(e.to_string()))?;
        
        Ok(json!({ "success": true, "brightness": params.brightness }))
    }
}

pub struct SetBrightnessParams {
    pub device_id: String,
    pub brightness: u8,
}// src/tools.rs
use rmcp::{tool, ToolHandler, McpError};
use crate::wled_client::WledClient;

// ========== GetDeviceInfo ==========
#[tool(description = "Get WLED device information and capabilities")]
pub struct GetDeviceInfo;

#[async_trait::async_trait]
impl ToolHandler for GetDeviceInfo {
    type Parameters = DeviceIdParams;
    type Result = serde_json::Value;

    async fn call(&self, params: Self::Parameters) -> Result<Self::Result, McpError> {
        let client = WledClient::new(&params.device_id)?;
        client.get_info().await
            .map_err(|e| McpError::internal_error(e.to_string()))
    }
}

// ========== GetState ==========
#[tool(description = "Get current WLED device state")]
pub struct GetState;

#[async_trait::async_trait]
impl ToolHandler for GetState {
    type Parameters = DeviceIdParams;
    type Result = serde_json::Value;

    async fn call(&self, params: Self::Parameters) -> Result<Self::Result, McpError> {
        let client = WledClient::new(&params.device_id)?;
        client.get_state().await
            .map_err(|e| McpError::internal_error(e.to_string()))
    }
}

// ========== SetBrightness ==========
#[tool(description = "Set the brightness of a WLED device")]
pub struct SetBrightness;

#[async_trait::async_trait]
impl ToolHandler for SetBrightness {
    type Parameters = SetBrightnessParams;
    type Result = serde_json::Value;

    async fn call(&self, params: Self::Parameters) -> Result<Self::Result, McpError> {
        let client = WledClient::new(&params.device_id)?;
        let state = WledState { bri: Some(params.brightness), ..Default::default() };
        client.set_state(&state).await
            .map_err(|e| McpError::internal_error(e.to_string()))?;
        Ok(json!({ "success": true, "brightness": params.brightness }))
    }
}

// ========== SetPower ==========
#[tool(description = "Turn WLED device on or off")]
pub struct SetPower;

#[async_trait::async_trait]
impl ToolHandler for SetPower {
    type Parameters = SetPowerParams;
    type Result = serde_json::Value;

    async fn call(&self, params: Self::Parameters) -> Result<Self::Result, McpError> {
        let client = WledClient::new(&params.device_id)?;
        let state = WledState { on: Some(params.on), ..Default::default() };
        client.set_state(&state).await
            .map_err(|e| McpError::internal_error(e.to_string()))?;
        Ok(json!({ "success": true, "power": params.on }))
    }
}

// ... (repeat pattern for SetColor, SetEffect, SetPalette, SetSegment, ListEffects, ListPalettes)

// ========== Common Parameter Types ==========
pub struct DeviceIdParams {
    pub device_id: String,
}

pub struct SetBrightnessParams {
    pub device_id: String,
    pub brightness: u8,
}

pub struct SetPowerParams {
    pub device_id: String,
    pub on: bool,
}

pub struct SetColorParams {
    pub device_id: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

// ... etc for each tool's specific params