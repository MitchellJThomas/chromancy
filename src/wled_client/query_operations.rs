impl WledClient {
    /// Get complete device state (on, brightness, segments, etc.)
    pub async fn get_state(&self) -> Result<WledState> {
        let url = format!("{}/state", self.base_url);
        let resp = self.client.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Get device info (firmware, LED count, capabilities, uptime)
    pub async fn get_info(&self) -> Result<WledInfo> {
        let url = format!("{}/info", self.base_url);
        let resp = self.client.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Get complete state + info in one call (more efficient)
    pub async fn get_full_state(&self) -> Result<WledFullState> {
        let url = format!("{}/", self.base_url);  // /json returns everything
        let resp = self.client.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Get list of available effects (by ID and name)
    pub async fn list_effects(&self) -> Result<Vec<Effect>> {
        let url = format!("{}/eff", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let names: Vec<String> = resp.json().await?;
        Ok(names
            .into_iter()
            .enumerate()
            .map(|(id, name)| Effect { id, name })
            .collect())
    }

    /// Get list of available palettes (by ID and name)
    pub async fn list_palettes(&self) -> Result<Vec<Palette>> {
        let url = format!("{}/pal", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let names: Vec<String> = resp.json().await?;
        Ok(names
            .into_iter()
            .enumerate()
            .map(|(id, name)| Palette { id, name })
            .collect())
    }

    /// Get detailed palette color data
    pub async fn get_palette_colors(&self, palette_id: usize) -> Result<PaletteColors> {
        let url = format!("{}/palx?id={}", self.base_url, palette_id);
        let resp = self.client.get(&url).send().await?;
        Ok(resp.json().await?)
    }

    /// Ping the device (check if reachable)
    pub async fn ping(&self) -> Result<bool> {
        let url = format!("{}/info", self.base_url);
        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}