use std::path::PathBuf;

use chromancy::{WledFleet, telemetry};
use chromancy::tools::ChromancyServer;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("wled-config.toml"));

    // Telemetry sends to OTLP (logs to stderr; stdout is reserved for MCP stdio).
    // If no OTLP endpoint is available the server still runs with fmt logging.
    let _guards = telemetry::init().unwrap_or_else(|e| {
        eprintln!("[chromancy] telemetry unavailable ({e}), using stderr logging");
        // Install a minimal stderr subscriber so tracing macros don't panic.
        let _ = tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .try_init();
        // Return a dummy guard by panicking into the error path;
        // re-raise as a fatal error.
        panic!("telemetry::init failed irrecoverably: {e}");
    });

    let fleet = WledFleet::load_from_config(&config_path)
        .map_err(|e| format!("Failed to load config '{}': {e}", config_path.display()))?;

    tracing::info!(
        config = %config_path.display(),
        groups = fleet.list_groups().len(),
        "Fleet loaded — starting MCP server"
    );

    let server = ChromancyServer { fleet };
    let transport = rmcp::transport::io::stdio();
    let running = server.serve(transport).await?;
    running.waiting().await?;

    Ok(())
}
