# cyclonedds-wasm

[![crates.io](https://img.shields.io/crates/v/cyclonedds-wasm.svg)](https://crates.io/crates/cyclonedds-wasm)

**Experimental.** A DDS-shaped API for WebAssembly, backed by a WebSocket connection instead of the native CycloneDDS C library (which does not compile to WASM).

> **This is not a DDS implementation.** It speaks JSON over WebSocket, not RTPS/CDR, so it does not interoperate with DDS participants on its own — a DDS-to-WebSocket bridge server is required. Only best-effort reliability and volatile durability are modelled. For real DDS, use the [`cyclonedds`](https://crates.io/crates/cyclonedds) crate on a native target.

## Install

```toml
[dependencies]
cyclonedds-wasm = "0.1"
```

Builds as both `cdylib` (for `wasm-bindgen`) and `rlib`. Target: `wasm32-unknown-unknown`.

## Usage

```rust,ignore
use cyclonedds_wasm::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MyMessage {
    id: i32,
    text: String,
}

let participant = WasmDomainParticipant::new("ws://localhost:8080/dds")?;
let topic = participant.create_topic::<MyMessage>("HelloWorld")?;
let writer = participant.create_writer(&topic)?;
writer.write(&MyMessage { id: 1, text: "hello".to_string() })?;
```

Public API: `WasmDomainParticipant` (`new`, `create_topic`, `create_writer`, `create_reader`, `disconnect`), `WasmTopic<T>`, `WasmDataWriter<T>` (`write`), `WasmDataReader<T>` (`topic_name`), and `WasmDdsError`/`WasmDdsResult<T>`. Sample types must implement `serde::Serialize` and/or `serde::de::DeserializeOwned` — the `DdsType` derive macros from the native crate do not apply here.

## Documentation

- [docs.rs/cyclonedds-wasm](https://docs.rs/cyclonedds-wasm)
- [Repository](https://github.com/mzet97/cyclonedds-rust)

## License

MIT — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT).
