// Quick start
let client = WledClient::new("192.168.1.101")?;

// Check if device is online
if client.ping().await? {
    println!("Device is reachable!");
}

// Get current state
let state = client.get_state().await?;
println!("Brightness: {}", state.bri);
println!("Power: {}", state.on);

// Get device info
let info = client.get_info().await?;
println!("Device: {} ({} LEDs)", info.name, info.leds.count);

// List available effects
let effects = client.list_effects().await?;
for effect in &effects {
    println!("  {}: {}", effect.id, effect.name);
}

// Set brightness
client.set_brightness(128).await?;

// Set color (warm white)
client.set_color(255, 200, 150).await?;

// Set effect
let rainbow = effects.iter().find(|e| e.name == "Rainbow").unwrap();
client.set_effect(rainbow.id).await?;

// Complex: multi-segment update
let state = WledStateRequest {
    seg: Some(vec![
        SegmentRequest {
            col: Some(vec![[255, 0, 0, 0]]),
            fx: Some(0),
            ..Default::default()
        },
        SegmentRequest {
            col: Some(vec![[0, 255, 0, 0]]),
            fx: Some(1),
            ..Default::default()
        },
    ]),
    ..Default::default()
};
client.set_state(&state).await?;