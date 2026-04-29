impl WledClient {
    /// Set power state (on/off)
    pub async fn set_power(&self, on: bool) -> Result<()> {
        self.set_state(&WledStateRequest {
            on: Some(on),
            ..Default::default()
        }).await
    }

    /// Set brightness (0-255)
    pub async fn set_brightness(&self, brightness: u8) -> Result<()> {
        self.set_state(&WledStateRequest {
            bri: Some(brightness),
            ..Default::default()
        }).await
    }

    /// Set primary color (RGB, affects first segment by default)
    pub async fn set_color(&self, r: u8, g: u8, b: u8) -> Result<()> {
        let segment = SegmentRequest {
            col: Some(vec![[r, g, b, 0]]),
            ..Default::default()
        };
        self.set_state(&WledStateRequest {
            seg: Some(vec![segment]),
            ..Default::default()
        }).await
    }

    /// Set active effect by ID
    pub async fn set_effect(&self, effect_id: usize) -> Result<()> {
        let segment = SegmentRequest {
            fx: Some(effect_id),
            ..Default::default()
        };
        self.set_state(&WledStateRequest {
            seg: Some(vec![segment]),
            ..Default::default()
        }).await
    }

    /// Set active palette by ID
    pub async fn set_palette(&self, palette_id: usize) -> Result<()> {
        let segment = SegmentRequest {
            pal: Some(palette_id),
            ..Default::default()
        };
        self.set_state(&WledStateRequest {
            seg: Some(vec![segment]),
            ..Default::default()
        }).await
    }

    /// Set transition duration (in deciseconds)
    pub async fn set_transition(&self, duration_ds: u16) -> Result<()> {
        self.set_state(&WledStateRequest {
            transition: Some(duration_ds),
            ..Default::default()
        }).await
    }

    /// Full state update (for complex multi-segment configurations)
    pub async fn set_state(&self, state: &WledStateRequest) -> Result<()> {
        let resp = self.client
            .post(&self.base_url)
            .json(state)
            .send()
            .await?;
        
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(WledError::Api {
                status: resp.status().as_u16(),
                message: resp.text().await.unwrap_or_default(),
            })
        }
    }

    /// Direct HTTP API (for endpoints not covered above)
    /// Example: /win&SB=128 for brightness via legacy API
    pub async fn raw_request(&self, endpoint: &str) -> Result<String> {
        let url = format!("http://{}/{}", 
            self.base_url.trim_end_matches("/json"),
            endpoint.trim_start_matches('/')
        );
        let resp = self.client.get(&url).send().await?;
        Ok(resp.text().await?)
    }
}