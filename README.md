# cyclonedds-rust

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE-MIT)
[![crates.io](https://img.shields.io/crates/v/cyclonedds.svg)](https://crates.io/crates/cyclonedds)

Safe, idiomatic Rust bindings for [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds) — a high-performance implementation of the OMG Data Distribution Service (DDS) specification.

> **Unofficial project.** This is a community binding, not affiliated with or endorsed by the Eclipse CycloneDDS project.

## Highlights

- **Complete DDS entity model** — DomainParticipant, Publisher, Subscriber, Topic, DataWriter, DataReader
- **26+ QoS policies** via a type-safe `QosBuilder` pattern
- **13 listener callbacks** via `ListenerBuilder` (data available, matched, liveliness, deadline, etc.)
- **WaitSet / ReadCondition / QueryCondition / GuardCondition** for event-driven architectures
- **Derive macros** for topic types: `DdsTypeDerive`, `DdsEnumDerive`, `DdsUnionDerive`, `DdsBitmaskDerive`
- **CDR serialization** (XCDR1/XCDR2), dynamic types, type discovery (XTypes)
- **Async Streams** (`read_aiter`, `take_aiter`) with tokio integration
- **Async Timeouts & Cancellation** (`read_aiter_timeout`, `take_aiter_timeout`) with safe cancellation
- **Zero-Copy Loans** (`request_loan`, `read_loan`, `take_loan`) for minimal latency
- **DDS Security** (`SecurityConfig`) with X.509 certificate validation and hot-reload
- **ROS2 Interop** helpers (`ros2_topic_name`, `ros2_qos_reliable`) for seamless ROS2 integration
- **Diagnostics CLI** (`diagnose`, `metrics`) with Prometheus export support
- **`tracing` Integration** for structured logs and distributed spans
- **WASM (Experimental)** — `cyclonedds-wasm` crate for browser-based DDS over WebSocket
- **no_std / Embedded (Experimental)** — define DDS types without `std` for embedded targets

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
cyclonedds = "3.0.0-alpha.3"
```

### Define a Topic Type

The recommended way to define a topic type is `#[derive(DdsTypeDerive)]`, which generates the `DdsType` trait implementation (including the mandatory `Native` associated type) for a plain struct:

```rust
use cyclonedds::DdsTypeDerive;

#[derive(DdsTypeDerive, Clone, Debug)]
struct HelloWorld {
    #[key]
    id: i32,
    message: String,
}
```

`String`/`Vec` fields are supported directly by the derive macro — it generates a wire-compatible `Native` representation (`DdsString`/`DdsSequence`) behind the scenes. See [Type System](docs/type-system.md) for `#[key]`, `#[dds_enum]`, and other attributes.

<details>
<summary>Advanced: manual <code>DdsType</code> implementation</summary>

For fixed-size, `#[repr(C)]` types you can implement `DdsType` by hand. Note the mandatory `type Native` associated type (added in 3.0.0-alpha.3) — for types with no heap-allocated fields it is simply `Self`:

```rust
use cyclonedds::{adr, adr_bst, DdsType, OP_FLAG_SGN, TYPE_4BY};

#[repr(C)]
struct HelloWorld {
    id: i32,
    message: [u8; 256],
}

impl DdsType for HelloWorld {
    type Native = Self;

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

</details>

### Publisher

```rust
use cyclonedds::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dp = DomainParticipant::new(0)?;
    let pub_ = Publisher::new(&dp)?;
    let topic = Topic::<HelloWorld>::new(&dp, "Hello")?;
    let writer = DataWriter::new(&pub_, &topic)?;
    let msg = HelloWorld { id: 1, message: "hello".into() };
    writer.write(&msg)?;
    Ok(())
}
```

### Subscriber

```rust
use cyclonedds::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dp = DomainParticipant::new(0)?;
    let sub = Subscriber::new(&dp)?;
    let topic = Topic::<HelloWorld>::new(&dp, "Hello")?;
    let reader = DataReader::<HelloWorld>::new(&sub, &topic)?;
    loop {
        for s in reader.take()? {
            println!("id={}", s.id);
        }
    }
}
```

> Runnable versions of these two examples live at [`cyclonedds/examples/pub.rs`](cyclonedds/examples/pub.rs) and [`cyclonedds/examples/sub.rs`](cyclonedds/examples/sub.rs) — they use the manual `impl DdsType` form with a fixed-size `[u8; 256]` message field:
>
> ```bash
> cargo run -p cyclonedds --example sub   # terminal 1
> cargo run -p cyclonedds --example pub   # terminal 2
> ```

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

Timeout/cancellation-aware variants (`read_aiter_timeout`, `take_aiter_timeout`) and batched variants (`read_aiter_batch`, `take_aiter_batch`, …) are also available on `DataReader` — see [`cyclonedds/src/async.rs`](cyclonedds/src/async.rs).

## `cyclonedds` Feature Flags

| Feature | Default | Enables | Pulls in |
|---------|:-------:|---------|----------|
| `std` | Yes (via `default`) | The full FFI-backed API — `DomainParticipant`, `Publisher`, `Subscriber`, `Topic`, `DataReader`/`DataWriter`, `Qos`, `Listener`, `WaitSet`, etc. | `cyclonedds-rust-sys`, `thiserror` |
| `async` | Yes (via `default`) | Async iterator methods on `DataReader` (`read_aiter`, `take_aiter`, `*_timeout`, `*_batch*`). Implies `std`. | `tokio` (rt, sync, time), `async-stream`, `futures-core`, `futures-util` |
| `security` | No | The `cyclonedds::security` module and `SecurityConfig` for DDS Security (authentication/access-control/crypto plugins). Implies `std` and forwards `security` to `cyclonedds-rust-sys`, which requires an OpenSSL-enabled CycloneDDS build. | — (feature-only; see [Requirements](#requirements)) |
| `tracing` | No | `#[tracing::instrument]`-style spans on DDS operations. | `tracing` |
| `serde` | No | `SerdeSample<T>`, a `DdsType` wrapper for any `T: Serialize + DeserializeOwned`, encoded with `postcard` (**not** OMG CDR — only interoperable between Rust nodes using the same wrapper). | `postcard`, `serde` |
| `opentelemetry` | No | `cyclonedds::observability::init_json_logging()` — a `tracing-subscriber` setup with JSON formatting and `RUST_LOG`-based env filtering. Implies `tracing`. | `tracing-subscriber` (fmt, json, env-filter) |
| `tokio-console` | No | tokio-console support for inspecting the async runtime. | `console-subscriber` |
| `no_std` | No | A `#![no_std]`-compatible subset: only `DdsType`, opcode constants (`adr`, `TYPE_*`, `OP_*`, …) and CDR-descriptor helpers — no networking, no `DomainParticipant`. Use with `default-features = false`. | — |

`default = ["std", "async"]`. Source: [`cyclonedds/Cargo.toml`](cyclonedds/Cargo.toml).

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
| IDL Compilation | Yes | Yes | **Yes** (`cyclonedds-idlc` / `cargo-cyclonedds`) |
| CLI Tools | Yes | No | **Yes** — see [`cyclonedds-cli` subcommands](#cli-examples) below |
| Async Streams (`read_aiter`, `take_aiter`) | No | No | **Yes** |
| Matched Endpoint Data | Yes | No | **Yes** |
| Zero-copy Write Loan | No | Yes | **Yes** (`request_loan`) |
| DDS Security | Yes | No | **Yes** (`SecurityConfig` + `--features security`) |
| Request-Reply Pattern | No | No | **Yes** (`Requester` / `Replier`) |
| Connection Pooling | No | No | **Yes** (`ParticipantPool`) |
| Serde Integration | No | No | **Yes** (`SerdeSample<T>` + `--features serde`) |
| WASM (Experimental) | No | No | **Yes** (`cyclonedds-wasm` crate) |
| no_std / Embedded (Experimental) | No | No | **Yes** (`--features no_std`) |

## Workspace Crates

Published (each of these has its own `readme.workspace = true` field — see [note below](#per-crate-readmes-and-cargo-publish)):

| Crate | Description |
|-------|-------------|
| [`cyclonedds-src`](https://crates.io/crates/cyclonedds-src) | Bundled CycloneDDS C source (build dependency) |
| [`cyclonedds-rust-sys`](https://crates.io/crates/cyclonedds-rust-sys) | Low-level, unsafe FFI bindings to the CycloneDDS C library |
| [`cyclonedds`](https://crates.io/crates/cyclonedds) | High-level, safe Rust API |
| [`cyclonedds-derive`](https://crates.io/crates/cyclonedds-derive) | Procedural derive macros (`DdsType`, `DdsEnum`, `DdsUnion`, `DdsBitmask`, re-exported by `cyclonedds` as `DdsTypeDerive`/`DdsEnumDerive`/`DdsUnionDerive`/`DdsBitmaskDerive`) |
| [`cyclonedds-build`](https://crates.io/crates/cyclonedds-build) | `build.rs` helper (`compile_idl`) for generating Rust types from IDL files |
| [`cyclonedds-idlc`](https://crates.io/crates/cyclonedds-idlc) | Standalone CLI that compiles IDL files to Rust source (wraps `cyclonedds-build`) |
| [`cyclonedds-cli`](https://crates.io/crates/cyclonedds-cli) | Command-line tools (`ls`, `ps`, `subscribe`, `bridge`, `perf`, `typeof`, `publish`, `discover`, `echo`, `record`, `replay`, `monitor`, `health`, `topology`, `diagnose`, `metrics`) |

Published, but without a `readme` field in `Cargo.toml` (crates.io will show no README for these unless one is added upstream):

| Crate | Description |
|-------|-------------|
| [`cargo-cyclonedds`](https://crates.io/crates/cargo-cyclonedds) | Cargo plugin (`cargo cyclonedds generate <idl>`) wrapping `cyclonedds-build` |
| [`cyclonedds-wasm`](https://crates.io/crates/cyclonedds-wasm) | Experimental WebAssembly bindings (JSON-over-WebSocket transport) |

**Not published** (`publish = false` in `Cargo.toml`, workspace-internal only):

| Crate | Description |
|-------|-------------|
| `cyclonedds-bench` | Criterion benchmarks (latency, throughput, CDR, IPC comparison) |
| `cyclonedds-test-suite` | Integration test suite |
| `dds-demo-test` | Ad-hoc demo/manual-test binaries (`comprehensive_test`, `publisher`, `subscriber`) |

**Not a workspace member**: `fuzz/` is its own `cargo-fuzz` workspace (it is not listed in the root `[workspace] members`) and is not published. See [Fuzzing](docs/fuzzing.md).

### Per-crate READMEs

Each of the 9 published crates has its own `README.md` and declares `readme = "README.md"` explicitly in its `Cargo.toml`, so crates.io and docs.rs render a crate-specific page.

Note that inheriting the field instead (`readme.workspace = true`) would *not* do this: Cargo resolves the inherited path relative to the workspace root, so every crate would render this root `README.md`. Keep the explicit per-crate value when adding a new published crate.

## Build

```bash
cargo build --workspace          # build everything
cargo test --workspace           # run tests
cargo build --workspace --release
```

### Requirements

- Rust 1.85+ (MSRV, `rust-version` in `[workspace.package]`)
- CMake 3.16+ (the vendored CycloneDDS `CMakeLists.txt` requires `cmake_minimum_required(VERSION 3.16)`)
- A C/C++ compiler (MSVC on Windows, gcc/clang on Linux/macOS)

> **Note:** Clang is not required for end users — prebuilt FFI bindings (`cyclonedds-rust-sys/src/prebuilt_bindings.rs`) ship with the crate. Clang is only needed by maintainers regenerating bindings (see `scripts/regenerate-bindings.sh`).

### How `cyclonedds-rust-sys` actually builds

`cyclonedds-rust-sys`'s `build.rs` does **not** run `bindgen` at build time — it uses the checked-in `prebuilt_bindings.rs` (generated on macOS/Linux, where `bindgen` resolves the CycloneDDS headers correctly) and links against a native CycloneDDS C library resolved in this order:

1. `CYCLONEDDS_SRC` environment variable, if set (path to a CycloneDDS source tree).
2. The bundled source shipped by the `cyclonedds-src` crate (used automatically when this workspace is built from a `cargo publish`'d/vendored copy).
3. `vendor/cyclonedds/` in the workspace root (local development checkout).
4. If none of the above exist, a system-installed `libddsc`/`ddsc.dll`/`ddsc.lib` (searched in standard Linux lib paths; on Windows this only applies if you set `CYCLONEDDS_BUILD` yourself).

Once a source tree is found, **CMake is required** to configure and build the bundled `ddsc` target (`-DBUILD_SHARED_LIBS`, `-DBUILD_TESTING=OFF`, `-DBUILD_IDLC=OFF`, `-DBUILD_DDSPERF=OFF`, `-DBUILD_EXAMPLES=OFF`, `-DENABLE_SECURITY`/`-DENABLE_SSL` driven by the `security` Cargo feature). If `cmake` is not on `PATH` and no source tree is available, the build falls back to searching for a system library only; if that also fails, the build still emits `bindings.rs` and a dummy `cargo:rustc-link-lib=dylib=ddsc` so `cargo check`/IDE tooling keeps working, but linking a real binary will fail.

**ABI probe (fails the build on mismatch):** after resolving bindings, `build.rs` compiles and runs a small C program against the *same* CycloneDDS headers used for linking, measuring the real `sizeof`/`offsetof` of `dds_sample_info_t` and related types on the current target. Rust `const` assertions in `cyclonedds-rust-sys/src/lib.rs` compare these measured values against the prebuilt bindings and **fail the build at compile time** if they disagree — this turns a class of silent memory-corruption bugs (stale prebuilt bindings vs. a different libc/compiler ABI) into a compile error.

**Cross-compilation:** the ABI probe can only *run* when `HOST == TARGET`. When cross-compiling, `build.rs` requires a pre-generated snapshot at `cyclonedds-rust-sys/abi/<target-triple>.rs` and **panics if it is missing**, with instructions to build natively for that target once and copy the resulting `abi_probe.rs` into place. As of this writing, `cyclonedds-rust-sys/abi/` does not exist in this repository — no snapshots are checked in yet, so cross-compiling any target where `HOST != TARGET` will fail until a maintainer builds one natively and commits the snapshot.

**System library note:** if no vendored/bundled source is available at all (only a system `libddsc`), `build.rs` explicitly panics — using a system-installed CycloneDDS without the vendored source tree is not a supported configuration in this workspace, because the ABI probe needs the matching headers.

### WSL Notes

If building in WSL, ensure `libddsc.so` is discoverable after the first build:

```bash
export LD_LIBRARY_PATH=~/cyclonedds-rust/vendor/cyclonedds/build/lib:$LD_LIBRARY_PATH
cargo test --workspace --features async
```

## CLI Examples

```bash
# List all topics in a domain
cargo run --bin cyclonedds-cli -- ls --domain-id 0

# Show participant status
cargo run --bin cyclonedds-cli -- ps --domain-id 0

# Subscribe to a topic
cargo run --bin cyclonedds-cli -- subscribe HelloWorld

# Subscribe with JSON output and filter
cargo run --bin cyclonedds-cli -- subscribe HelloWorld --json --filter "id > 10"

# Show type info
cargo run --bin cyclonedds-cli -- typeof HelloWorld

# Publish at 10 Hz
cargo run --bin cyclonedds-cli -- publish HelloWorld --message "hi" --rate 10

# Monitor throughput
cargo run --bin cyclonedds-cli -- monitor HelloWorld

# Health check
cargo run --bin cyclonedds-cli -- health "HelloWorld,AnotherTopic"

# Generate topology graph
cargo run --bin cyclonedds-cli -- topology --output topology.dot

# Subscribe to multiple topics simultaneously
cargo run --bin cyclonedds-cli -- subscribe --topics "TopicA,TopicB" --json

# Bridge samples from one topic to another (optionally across domains)
cargo run --bin cyclonedds-cli -- bridge TopicA TopicB --domain-src 0 --domain-dst 1

# Full domain state as JSON
cargo run --bin cyclonedds-cli -- diagnose --domain-id 0

# Prometheus-compatible metrics for a topic
cargo run --bin cyclonedds-cli -- metrics HelloWorld
```

All 16 subcommands (`ls`, `ps`, `subscribe`, `bridge`, `perf`, `typeof`, `publish`, `discover`, `echo`, `record`, `replay`, `monitor`, `health`, `topology`, `diagnose`, `metrics`) are defined in [`cyclonedds-cli/src/main.rs`](cyclonedds-cli/src/main.rs); run `cargo run --bin cyclonedds-cli -- --help` or `... <subcommand> --help` for the authoritative, up-to-date flag list.

## Examples

The `cyclonedds` crate ships runnable examples under [`cyclonedds/examples/`](cyclonedds/examples/):

```bash
# Terminal 1 - subscriber
cargo run -p cyclonedds --example sub

# Terminal 2 - publisher
cargo run -p cyclonedds --example pub
```

Other examples in that directory: `pub_keyed`, `pub_qos`, `sub_qos`, `sub_async`, `metrics`, `no_std_types`, `request_reply_calc`, `ros2_turtlesim`, `shm_pub`, `shm_sub`. Cross-process interop examples (`interop_pub`, `interop_sub`) and a latency micro-benchmark (`bench_latency`) live in [`cyclonedds-test-suite/examples/`](cyclonedds-test-suite/examples/).

## Documentation

- [Getting Started](docs/getting-started.md) — installation, first steps, WSL notes
- [Tutorial](docs/tutorial.md) — step-by-step first DDS application
- [API Guide](docs/api-guide.md) — tour of all major API features
- [Type System](docs/type-system.md) — `DdsType` derive, supported types, CDR encoding
- [QoS Reference](docs/qos-reference.md) — all QoS policies and builder patterns
- [ROS2 Integration](docs/ros2-integration.md) — communicating with ROS2 nodes
- [Security Guide](docs/security-guide.md) — DDS Security setup and certificates
- [Observability](docs/observability.md) — `tracing` integration for structured logs and spans
- [Benchmarks](docs/benchmarks.md) — running performance benchmarks and comparisons
- [Fuzzing](docs/fuzzing.md) — automated fuzz testing with `cargo-fuzz`
- [FAQ](docs/faq.md) — frequently asked questions and troubleshooting
- [Migration from Python](docs/migration-from-python.md) — guide for `cyclonedds-python` users

Also in `docs/`: [Architecture](docs/architecture.md) — layer and data-flow diagrams; [Async Patterns](docs/async-patterns.md); [Security in Production](docs/security-production.md).

## WASM Support (Experimental)

The `cyclonedds-wasm` crate provides a DDS-compatible API for WebAssembly:

```toml
[dependencies]
cyclonedds-wasm = "0.1"
```

```rust
use cyclonedds_wasm::*;

let participant = WasmDomainParticipant::new("ws://localhost:8080/dds").unwrap();
let topic = participant.create_topic::<MyMessage>("HelloWorld").unwrap();
let writer = participant.create_writer(&topic).unwrap();
writer.write(&MyMessage { id: 1, text: "hello".to_string() }).unwrap();
```

> **Note:** This is not a full DDS implementation. It uses JSON over WebSocket rather than RTPS/CDR. A DDS-to-WebSocket bridge is required to communicate with native DDS participants.

## Embedded / no_std (Experimental)

When the `no_std` feature is enabled, `cyclonedds` exports only pure-Rust DDS types and constants without the CycloneDDS C FFI:

```toml
[dependencies]
cyclonedds = { version = "3.0.0-alpha.3", default-features = false, features = ["no_std"] }
```

```rust
#![no_std]
extern crate alloc;

use cyclonedds::{adr, DdsType, OP_RTS, TYPE_4BY};

#[repr(C)]
pub struct SensorReading {
    pub sensor_id: i32,
    pub temperature: f32,
}

impl DdsType for SensorReading {
    type Native = Self;

    fn type_name() -> &'static str {
        "SensorReading"
    }
    fn ops() -> alloc::vec::Vec<u32> {
        let mut ops = alloc::vec::Vec::new();
        ops.extend(adr(TYPE_4BY | (1 << 2), 0)); // sensor_id @ offset 0
        ops.extend(adr(TYPE_4BY, 4));            // temperature @ offset 4
        ops.push(OP_RTS);
        ops
    }
}
```

This is useful for defining DDS-compatible types on embedded systems (e.g., `thumbv7em-none-eabihf`) where the full CycloneDDS C library cannot run. Actual CDR serialization must be performed manually or via a separate no-std serializer.

The example mirrors [`cyclonedds/examples/no_std_types.rs`](cyclonedds/examples/no_std_types.rs) (`cargo run -p cyclonedds --example no_std_types`), which exercises the type-descriptor logic on a `std` host. Note that the `no_std` path has not been compiled against a real embedded target — treat it as experimental.

## Known Limitations

- **CLI `publish`:** Supports string messages, JSON payloads, and dynamic types discovered at runtime. Complex nested structs may require using the Rust API directly for full control.
- **DDS Security on Windows:** Requires OpenSSL to be installed and `OPENSSL_ROOT_DIR` configured. The `security` feature is disabled by default on Windows CI to avoid build issues.
- **WASM:** Not a full DDS implementation — requires a WebSocket bridge to communicate with native DDS.
- **no_std:** No actual DDS networking. Only type definitions and CDR opcode constants are available.
- **Cross-compilation:** requires a pre-generated ABI snapshot under `cyclonedds-rust-sys/abi/<target-triple>.rs` (see [Build](#build) above); none are currently checked into this repository.

## Benchmarks

```bash
cargo test -p cyclonedds-test-suite --test write_loan     # zero-copy write test
cargo test -p cyclonedds-test-suite --test interop         # cross-process pub/sub test
cargo run -p cyclonedds-test-suite --example interop_pub   # standalone publisher
cargo run -p cyclonedds-test-suite --example interop_sub   # standalone subscriber
```

See [`cyclonedds-test-suite/`](cyclonedds-test-suite/) for these tests/examples and [`cyclonedds-bench/`](cyclonedds-bench/) for the Criterion benchmark suite (`cargo bench -p cyclonedds-bench`). Neither crate is published to crates.io.

## License

Licensed under the [MIT License](LICENSE-MIT).

## Acknowledgments

Built on [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds) — a high-performance DDS implementation.
