//! OpenTelemetry initialisation for Chromancy.
//!
//! Sends traces, metrics, and logs to any OTLP-compatible collector via gRPC.
//! SigNoz, Jaeger, Grafana Tempo, and the OpenTelemetry Collector are all
//! supported by pointing `OTEL_EXPORTER_OTLP_ENDPOINT` at the right host.
//!
//! # Grafana LGTM quick-start
//!
//! ```sh
//! # Start the all-in-one LGTM stack (Grafana, Tempo, Loki, Prometheus)
//! podman compose -f docker-compose.lgtm.yml up
//!
//! # Point chromancy at it
//! export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
//! export OTEL_SERVICE_NAME=chromancy          # optional override
//! export RUST_LOG=info
//! ./chromancy wled-config.toml
//! ```
//!
//! After a few tool calls you will see traces in Grafana at
//! `http://localhost:3000` (Explore → Tempo).

use std::time::Duration;

use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    logs::LoggerProvider,
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime,
    trace::TracerProvider,
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::error::WledError;

// ── Metrics instruments ───────────────────────────────────────────────────────

/// Pre-built OpenTelemetry metric instruments used across the codebase.
/// Instruments are created once at startup and are cheaply cloneable.
#[derive(Clone)]
pub struct Metrics {
    /// Total MCP tool calls (attributes: `tool`, `success`).
    pub tool_calls_total: opentelemetry::metrics::Counter<u64>,
    /// Total WLED HTTP requests (attributes: `device`, `operation`).
    pub wled_requests_total: opentelemetry::metrics::Counter<u64>,
    /// WLED HTTP request duration in seconds (attributes: `device`, `operation`).
    pub wled_request_duration_seconds: opentelemetry::metrics::Histogram<f64>,
    /// Sync health check results (attributes: `group`, `healthy`).
    pub sync_health_checks_total: opentelemetry::metrics::Counter<u64>,
}

impl Metrics {
    pub fn new() -> Self {
        let meter = global::meter("chromancy");
        Self {
            tool_calls_total: meter
                .u64_counter("mcp.tool.calls.total")
                .with_description("Total number of MCP tool invocations")
                .build(),
            wled_requests_total: meter
                .u64_counter("wled.requests.total")
                .with_description("Total HTTP requests sent to WLED devices")
                .build(),
            wled_request_duration_seconds: meter
                .f64_histogram("wled.request.duration.seconds")
                .with_description("WLED HTTP request latency")
                .with_unit("s")
                .build(),
            sync_health_checks_total: meter
                .u64_counter("wled.sync.health.checks.total")
                .with_description("Total sync health checks performed")
                .build(),
        }
    }

    /// Record one tool call.
    pub fn record_tool_call(&self, tool: &str, success: bool) {
        self.tool_calls_total.add(
            1,
            &[
                KeyValue::new("tool", tool.to_string()),
                KeyValue::new("success", success),
            ],
        );
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

// ── Shutdown guard ────────────────────────────────────────────────────────────

/// Holds OTel provider handles. Flush + shutdown is triggered on `Drop`.
pub struct TelemetryGuards {
    tracer_provider: TracerProvider,
    meter_provider: SdkMeterProvider,
    logger_provider: LoggerProvider,
}

impl TelemetryGuards {
    /// Create a no-op guard for when telemetry is disabled.
    /// Shutdown is a no-op on these empty providers.
    pub fn noop() -> Self {
        Self {
            tracer_provider: TracerProvider::builder().build(),
            meter_provider: SdkMeterProvider::builder().build(),
            logger_provider: LoggerProvider::builder().build(),
        }
    }
}

impl Drop for TelemetryGuards {
    fn drop(&mut self) {
        if let Err(e) = self.tracer_provider.shutdown() {
            eprintln!("[otel] tracer shutdown error: {e}");
        }
        if let Err(e) = self.meter_provider.shutdown() {
            eprintln!("[otel] meter shutdown error: {e}");
        }
        if let Err(e) = self.logger_provider.shutdown() {
            eprintln!("[otel] logger shutdown error: {e}");
        }
    }
}

// ── Init ──────────────────────────────────────────────────────────────────────

/// Initialise OpenTelemetry and the `tracing` subscriber.
///
/// Reads the OTLP endpoint from the environment variable
/// `OTEL_EXPORTER_OTLP_ENDPOINT` (default: `http://localhost:4317`).
///
/// **Must be called before any `tracing!` macros are used.**
pub fn init() -> Result<TelemetryGuards, WledError> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let service_name = std::env::var("OTEL_SERVICE_NAME")
        .unwrap_or_else(|_| "chromancy".to_string());

    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, service_name),
        KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
    ]);

    // ── Traces ────────────────────────────────────────────────────────────────
    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .with_timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| WledError::ConfigError(format!("OTel trace exporter: {e}")))?;

    let tracer_provider = TracerProvider::builder()
        .with_batch_exporter(span_exporter, runtime::Tokio)
        .with_resource(resource.clone())
        .build();

    global::set_tracer_provider(tracer_provider.clone());
    let tracer = tracer_provider.tracer("chromancy");

    // ── Metrics ───────────────────────────────────────────────────────────────
    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .with_timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| WledError::ConfigError(format!("OTel metric exporter: {e}")))?;

    let interval = std::env::var("OTEL_METRIC_EXPORT_INTERVAL")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(30));

    let reader = PeriodicReader::builder(metric_exporter, runtime::Tokio)
        .with_interval(interval)
        .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource.clone())
        .build();

    global::set_meter_provider(meter_provider.clone());

    // ── Logs ──────────────────────────────────────────────────────────────────
    let log_exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(&endpoint)
        .with_timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| WledError::ConfigError(format!("OTel log exporter: {e}")))?;

    let logger_provider = LoggerProvider::builder()
        .with_batch_exporter(log_exporter, runtime::Tokio)
        .with_resource(resource)
        .build();

    // ── tracing subscriber ────────────────────────────────────────────────────
    // Logs go to stderr (stdout is reserved for the MCP stdio transport).
    let otel_trace_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let otel_log_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("chromancy=info,warn")),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(false),
        )
        .with(otel_trace_layer)
        .with(otel_log_layer)
        .init();

    tracing::info!(
        endpoint = %endpoint,
        "OpenTelemetry initialised — sending to OTLP collector"
    );

    Ok(TelemetryGuards {
        tracer_provider,
        meter_provider,
        logger_provider,
    })
}
