# cyclonedds-rust

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)

Safe, idiomatic Rust bindings for [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds) — a high-performance implementation of the OMG Data Distribution Service (DDS) specification.

## Highlights

- **Complete DDS entity model** — DomainParticipant, Publisher, Subscriber, Topic, DataWriter, DataReader
- **26+ QoS policies** via a type-safe `QosBuilder` pattern
- **13 listener callbacks** via `ListenerBuilder` (data available, matched, liveliness, deadline, etc.)
- **WaitSet / ReadCondition / QueryCondition / GuardCondition** for event-driven architectures
- **Derive macros** for topic types: `DdsType`, `DdsEnum`, `DdsUnion`, `DdsBitmask`
- **CDR serialization** (XCDR1/XCDR2), dynamic types, type discovery (XTypes)
- **Async Streams** (`read_aiter`, `take_aiter`) with tokio integration

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
cyclonedds = "1.5"
```

### Define a Topic Type

```rust
use cyclonedds::*;

#[repr(C)]
struct HelloWorld {
    id: i32,
    message: [u8; 256],
}

impl DdsType for HelloWorld {
    fn type_name() -> &'static str {
        "HelloWorld"
    }
    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 0));
        ops.extend(adr_bst(4, 256));
        ops
    }
}
```

### Publisher

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dp = DomainParticipant::new(0)?;
    let pub_ = Publisher::new(dp.entity())?;
    let topic = Topic::<HelloWorld>::new(dp.entity(), "Hello")?;
    let writer = DataWriter::new(pub_.entity(), topic.entity())?;
    let mut msg = HelloWorld { id: 1, message: [0; 256] };
    msg.message[..5].copy_from_slice(b"hello");
    writer.write(&msg)?;
    Ok(())
}
```

### Subscriber

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dp = DomainParticipant::new(0)?;
    let sub = Subscriber::new(dp.entity())?;
    let topic = Topic::<HelloWorld>::new(dp.entity(), "Hello")?;
    let reader = DataReader::<HelloWorld>::new(sub.entity(), topic.entity())?;
    loop {
        for s in reader.take()? {
            println!("id={}", s.id);
        }
    }
}
```

## Async Streams

When the `async` feature is enabled (default), `DataReader` provides async iterators over incoming samples:

```rust
use cyclonedds::DataReader;
use futures_util::StreamExt;

async fn consume<T: cyclonedds::DdsType>(reader: &DataReader<T>) {
    let mut stream = Box::pin(reader.read_aiter());
    while let Some(batch) = stream.next().await {
        match batch {
            Ok(samples) => println!("got {} samples", samples.len()),
            Err(e) => eprintln!("read error: {}", e),
        }
    }
}
```

## Feature Matrix

| Feature | Python (CycloneDDS) | .NET | Rust (this crate) |
|---------|---------------------|------|-------------------|
| Core Entities | Yes | Partial | **Yes** |
| QoS (26+) | Yes | Partial | **Yes** |
| Listeners (13) | Yes | Partial | **Yes** |
| WaitSet / Conditions | Yes | No | **Yes** |
| CDR Serialization (XCDR1/2) | Yes | Yes | **Yes** |
| Dynamic Types & Data | Yes | No | **Yes** |
| Type Discovery (XTypes) | Yes | No | **Yes** |
| Content-Filtered Topics | Yes | Partial | **Yes** (closure-based) |
| Union / Bitmask / Enum | Yes | Partial | **Yes** |
| IDL Compilation | Yes | Yes | **Yes** |
| CLI Tools | Yes | No | **Yes** (`ls`, `ps`, `subscribe`, `typeof`, `publish`, `discover`, `echo`, `record`, `replay`, `monitor`, `health`, `topology`) |
| Async Streams (`read_aiter`, `take_aiter`) | No | No | **Yes** |
| Matched Endpoint Data | Yes | No | **Yes** |
| Zero-copy Write Loan | No | Yes | **Yes** |
| DDS Security | Yes | No | **Yes** (`SecurityConfig` + `--features security`) |

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `cyclonedds-sys` | Low-level FFI bindings (generated via bindgen) |
| `cyclonedds` | High-level safe Rust API |
| `cyclonedds-derive` | Procedural derive macros (`DdsType`, `DdsEnum`, `DdsUnion`, `DdsBitmask`) |
| `cyclonedds-build` | Build-time helpers for generating types from IDL |
| `cyclonedds-idlc` | IDL compiler backend producing Rust source from IDL files |
| `cyclonedds-cli` | Command-line tools (`ls`, `ps`, `subscribe`, `typeof`, `publish`, `perf`, `discover`, `echo`, `record`, `replay`, `monitor`, `health`, `topology`) |
| `cargo-cyclonedds` | Cargo plugin (`cargo cyclonedds generate <idl>`) |
| `cyclonedds-bench` | Criterion benchmarks (latency, throughput, CDR) |
| `cyclonedds-test-suite` | Integration tests |

## Build

```bash
cargo build --workspace          # build everything
cargo test --workspace           # run tests
cargo build --workspace --release
```

### Requirements

- Rust 1.85+ (MSRV)
- CMake 3.10+
- C/C++ compiler

> **Note:** Clang is no longer required for end users. Prebuilt FFI bindings are shipped with the crate. Clang is only needed if you are a maintainer regenerating bindings (see `scripts/regenerate-bindings.sh`).

The bundled CycloneDDS source in `cyclonedds-src` is built automatically by `cyclonedds-rust-sys` when CMake is available.

### WSL Notes

If building in WSL, ensure `libddsc.so` is discoverable after the first build:

```bash
export LD_LIBRARY_PATH=~/cyclonedds-rust/vendor/cyclonedds/build/lib:$LD_LIBRARY_PATH
cargo test --workspace --features async
```

## CLI Examples

```bash
# List all topics in a domain
cargo run --bin cyclonedds-cli -- ls --domain 0

# Show participant status
cargo run --bin cyclonedds-cli -- ps --domain 0

# Subscribe to a topic
cargo run --bin cyclonedds-cli -- subscribe --topic HelloWorld

# Subscribe with JSON output and filter
cargo run --bin cyclonedds-cli -- subscribe --topic HelloWorld --json --filter "id > 10"

# Show type info
cargo run --bin cyclonedds-cli -- typeof --topic HelloWorld

# Publish at 10 Hz
cargo run --bin cyclonedds-cli -- publish --topic HelloWorld --message "hi" --rate 10

# Monitor throughput
cargo run --bin cyclonedds-cli -- monitor --topic HelloWorld

# Health check
cargo run --bin cyclonedds-cli -- health "HelloWorld,AnotherTopic"

# Generate topology graph
cargo run --bin cyclonedds-cli -- topology --output topology.dot
```

## Examples

```bash
# Terminal 1 - subscriber
cargo run --example sub

# Terminal 2 - publisher
cargo run --example pub
```

## Documentation

- [Getting Started](docs/getting-started.md) — installation, first steps, WSL notes
- [Tutorial](docs/tutorial.md) — step-by-step first DDS application
- [API Guide](docs/api-guide.md) — tour of all major API features
- [Type System](docs/type-system.md) — `DdsType` derive, supported types, CDR encoding
- [QoS Reference](docs/qos-reference.md) — all QoS policies and builder patterns
- [ROS2 Integration](docs/ros2-integration.md) — communicating with ROS2 nodes
- [Security Guide](docs/security-guide.md) — DDS Security setup and certificates
- [FAQ](docs/faq.md) — frequently asked questions and troubleshooting
- [Migration from Python](docs/migration-from-python.md) — guide for `cyclonedds-python` users

## Known Limitations

- **CLI `publish`:** Supports string messages, JSON payloads, and dynamic types discovered at runtime. Complex nested structs may require using the Rust API directly for full control.
- **DDS Security on Windows:** Requires OpenSSL to be installed and `OPENSSL_ROOT_DIR` configured. The `security` feature is disabled by default on Windows CI to avoid build issues.

## Benchmarks

```bash
cargo test --test write_loan     # zero-copy write test
cargo test --test interop        # cross-process pub/sub test
cargo run --example interop_pub  # standalone publisher
cargo run --example interop_sub  # standalone subscriber
```

## License

Licensed under the [MIT License](LICENSE-MIT).

## Acknowledgments

Built on [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds) — a high-performance DDS implementation.
