# Observability with `tracing`

The `cyclonedds` crate optionally integrates with the [`tracing`](https://docs.rs/tracing) ecosystem for structured logging and distributed tracing.

## Enabling tracing

Add the `tracing` feature to your `Cargo.toml`:

```toml
[dependencies]
cyclonedds = { version = "1.5", features = ["tracing"] }
```

## Instrumented operations

When the `tracing` feature is enabled, the following DDS operations are automatically instrumented with `#[tracing::instrument]` spans:

- **Entity creation**
  - `DomainParticipant::new`
  - `DomainParticipant::with_qos`
  - `DomainParticipant::with_listener`
  - `DomainParticipant::with_qos_and_listener`
  - `DynamicTypeBuilder::build`

- **DataWriter**
  - `DataWriter::write`
  - `DataWriter::write_dispose`
  - `DataWriter::write_loan_async`

- **DataReader**
  - `DataReader::read`
  - `DataReader::take`
  - `DataReader::take_async`
  - `DataReader::read_aiter`
  - `DataReader::read_aiter_batch`
  - `DataReader::take_aiter`

- **WaitSet**
  - `WaitSet::wait_async`

- **Type discovery**
  - `discover_type_from_type_info`

## Example with `tokio` subscriber

```rust
use tracing_subscriber;

fn main() {
    tracing_subscriber::fmt::init();

    let participant = cyclonedds::DomainParticipant::new(0).unwrap();
    // Spans are emitted automatically for instrumented operations.
}
```

Run with `RUST_LOG=info` to see DDS spans:

```bash
RUST_LOG=info cargo run --example pub --features tracing
```

## Custom subscribers

`tracing` is compatible with OpenTelemetry, Jaeger, Zipkin, and many other backends. To export DDS spans to OpenTelemetry:

```toml
[dependencies]
tracing = "0.1"
tracing-opentelemetry = "0.24"
opentelemetry = "0.24"
opentelemetry-jaeger = "0.21"
```

```rust
use opentelemetry::trace::TracerProvider;
use tracing_subscriber::layer::SubscriberExt;

fn init_tracing() {
    let provider = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("dds-app")
        .build_simple()
        .expect("jaeger init");

    let tracer = provider.tracer("dds-app");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = tracing_subscriber::Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber).unwrap();
}
```

## Performance notes

- The `tracing` feature is **disabled by default** to avoid runtime overhead when not needed.
- When enabled, `tracing` uses zero-cost macros; spans are only recorded if a subscriber is registered.
- For high-throughput scenarios, consider using `tracing::Level::DEBUG` or `TRACE` for write/read operations to avoid log flooding.
