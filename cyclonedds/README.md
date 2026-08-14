# cyclonedds

[![crates.io](https://img.shields.io/crates/v/cyclonedds.svg)](https://crates.io/crates/cyclonedds)
[![docs.rs](https://img.shields.io/docsrs/cyclonedds)](https://docs.rs/cyclonedds)

Safe, idiomatic Rust API for [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds). This is the crate most users of the [`cyclonedds-rust`](https://github.com/mzet97/cyclonedds-rust) workspace should depend on directly.

It wraps the unsafe FFI surface of [`cyclonedds-rust-sys`](https://crates.io/crates/cyclonedds-rust-sys) with safe entity types (`DomainParticipant`, `Publisher`, `Subscriber`, `Topic`, `DataWriter`, `DataReader`), a type-safe `QosBuilder`, `ListenerBuilder`, `WaitSet`/conditions, async iterator support, zero-copy loans, DDS Security, and more. See the [workspace README](https://github.com/mzet97/cyclonedds-rust#readme) for the full feature list, feature-flag matrix, and build requirements (CMake 3.16+, a C/C++ compiler).

## Install

```toml
[dependencies]
cyclonedds = "2.0"
```

## Minimal example

Topic types are usually defined with `#[derive(DdsTypeDerive)]`:

```rust
use cyclonedds::*;

#[derive(DdsTypeDerive, Clone, Debug)]
struct HelloWorld {
    #[key]
    id: i32,
    message: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dp = DomainParticipant::new(0)?;
    let publisher = Publisher::new(dp.entity())?;
    let topic = Topic::<HelloWorld>::new(dp.entity(), "Hello")?;
    let writer = DataWriter::new(publisher.entity(), topic.entity())?;
    writer.write(&HelloWorld { id: 1, message: "hello".into() })?;
    Ok(())
}
```

Runnable versions of this pattern live in [`examples/`](examples/) (`pub.rs`, `sub.rs`, `metrics.rs`, `pub_keyed.rs`, `sub_async.rs`, and more) — run with `cargo run -p cyclonedds --example <name>`.

## Feature flags

`default = ["std", "async"]`.

| Feature | Adds |
|---------|------|
| `std` | The full entity API (requires `cyclonedds-rust-sys`) |
| `async` | `read_aiter`/`take_aiter` async streams on `DataReader`, with timeout/cancellation and batched variants |
| `security` | `SecurityConfig` and the `cyclonedds::security` module (needs an OpenSSL-enabled CycloneDDS build) |
| `tracing` | `tracing`-instrumented spans on DDS operations |
| `serde` | `SerdeSample<T>`, a `postcard`-encoded `DdsType` wrapper for any `Serialize + DeserializeOwned` type |
| `opentelemetry` | `cyclonedds::observability::init_json_logging()` (JSON structured logging via `tracing-subscriber`) |
| `tokio-console` | tokio-console instrumentation |
| `no_std` | Pure-Rust `DdsType`/opcode definitions only, no networking — use with `default-features = false` |

Full descriptions and rationale: [workspace README § Feature Flags](https://github.com/mzet97/cyclonedds-rust#cyclonedds-feature-flags).

## Documentation

- [docs.rs/cyclonedds](https://docs.rs/cyclonedds) — generated API reference
- [Getting Started](https://github.com/mzet97/cyclonedds-rust/blob/main/docs/getting-started.md)
- [Type System](https://github.com/mzet97/cyclonedds-rust/blob/main/docs/type-system.md) — `DdsType` derive and attributes
- [QoS Reference](https://github.com/mzet97/cyclonedds-rust/blob/main/docs/qos-reference.md)
- [Repository](https://github.com/mzet97/cyclonedds-rust)

## License

MIT — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT).
