//! Observability helpers for `tracing`, OpenTelemetry, and tokio-console.
//!
//! This module provides convenience functions for setting up structured
//! logging and distributed tracing in DDS applications.
//!
//! # Features
//!
//! - **`tracing`** -- enables `#[tracing::instrument]` spans on DDS operations.
//! - **`opentelemetry`** -- re-exports `tracing-opentelemetry` and `opentelemetry-otlp`
//!   so you can wire up OTLP export in your application.
//! - **`tokio-console`** -- exposes tokio tasks to the tokio-console debugger.
//!
//! # OpenTelemetry Setup
//!
//! ```no_run
//! use cyclonedds::observability::init_json_logging;
//!
//! #[tokio::main]
//! async fn main() {
//!     init_json_logging();
//!     // Configure your OTLP exporter here using opentelemetry-otlp
//!     // ... DDS code
//! }
//! ```

/// Initialize a `tracing-subscriber` with JSON formatting and env-filter.
///
/// Reads `RUST_LOG` for filter configuration.
#[cfg(feature = "opentelemetry")]
pub fn init_json_logging() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .init();
}

/// Initialize tokio-console support.
///
/// Run with `TOKIO_CONSOLE_BIND=127.0.0.1:6669` to customize the listener address.
#[cfg(feature = "tokio-console")]
pub fn init_tokio_console() {
    console_subscriber::init();
}

/// Initialize JSON logging + tokio-console when both features are enabled.
#[cfg(all(feature = "opentelemetry", feature = "tokio-console"))]
pub fn init_full_observability() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let console_layer = console_subscriber::spawn();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer().json())
        .with(console_layer)
        .init();
}
