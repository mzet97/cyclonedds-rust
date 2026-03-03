# cyclonedds-rust

Rust bindings for [CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds) - an implementation of the OMG Data Distribution Service (DDS) specification.

## Overview

This crate provides a safe, idiomatic Rust API for CycloneDDS. It consists of:

- **cyclonedds-sys**: Low-level FFI bindings to the CycloneDDS C library
- **cyclonedds**: High-level safe Rust wrappers with RAII types and error handling

## Requirements

### Build Dependencies

- **Rust** (1.70+)
- **CMake** (3.10+)
- **Clang** (for bindgen)
- **C/C++ compiler** (MSVC on Windows, GCC/Clang on Linux)

### Runtime Dependencies

- CycloneDDS is built automatically as part of the crate build process

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
cyclonedds = "0.1"
```

## Quick Start

```rust
use cyclonedds::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a DomainParticipant
    let participant = DomainParticipant::new(0)?;

    // Create a topic
    let topic: Topic<MyData> = participant.create_topic("my_topic")?;

    // Create a publisher and writer
    let publisher = participant.create_publisher()?;
    let writer: DataWriter<MyData> = publisher.create_writer(&topic)?;

    // Publish data
    writer.write(&MyData { value: 42 })?;

    Ok(())
}
```

## Examples

Run the publisher example:

```bash
cargo run --example pub
```

Run the subscriber example in another terminal:

```bash
cargo run --example sub
```

## Features

- **Safe FFI**: All bindings are wrapped in safe Rust types
- **RAII**: Resources are automatically cleaned up using the Drop trait
- **Error handling**: Comprehensive error types using `thiserror`
- **Serialization support**: Works with `serde` for data serialization

## Building

```bash
# Build the workspace
cargo build --workspace

# Run tests
cargo test --workspace

# Build release
cargo build --workspace --release
```

## Platform Support

- **Linux/WSL**: Primary development target
- **Windows**: Partial support (build system may require adjustments)

## License

Licensed under either of:
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

## Contributing

Contributions are welcome! Please ensure tests pass before submitting PRs.
