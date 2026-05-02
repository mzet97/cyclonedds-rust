# Frequently Asked Questions

## General

### What is DDS?
Data Distribution Service (DDS) is a middleware standard for data-centric publish-subscribe communication. It provides discovery, reliability, quality of service, and real-time performance.

### Why use cyclonedds-rust instead of raw FFI?
This crate provides safe, idiomatic Rust bindings with:
- Type-safe DDS entities (no raw integer handles)
- Derive macros for topic types
- Async stream support
- Comprehensive QoS builder

## Installation

### Build fails with "CMake not found"
Install CMake 3.10+ and ensure it's in your PATH.

### Build fails on Windows with OpenSSL errors
DDS Security requires OpenSSL. Either:
- Install OpenSSL and set `OPENSSL_ROOT_DIR`
- Build without security: `cargo build --no-default-features`

### WSL build fails at link time
Set the library path:
```bash
export LD_LIBRARY_PATH=~/cyclonedds-rust/vendor/cyclonedds/build/lib:$LD_LIBRARY_PATH
```

## Usage

### How do I define a topic type with a key?
Use the `#[key]` attribute:
```rust
#[derive(DdsTypeDerive)]
struct SensorData {
    #[key]
    sensor_id: i32,
    value: f64,
}
```

### How do I use async/await with DDS?
Enable the `async` feature (default) and use:
```rust
let mut stream = Box::pin(reader.read_aiter());
while let Some(batch) = stream.next().await {
    // process batch
}
```

### How do I connect to a ROS2 node?
ROS2 uses DDS under the hood. Define matching types and use the same topic names. See [ROS2 Integration](ros2-integration.md).

### Can I use DDS across the network?
Yes. By default, CycloneDDS uses multicast for discovery and UDP for data. Configure `CYCLONEDDS_URI` with an XML config for specific network interfaces.

### How do I debug discovery issues?
Use the CLI tools:
```bash
cyclonedds-cli ls       # list entities
cyclonedds-cli ps       # list participants
cyclonedds-cli typeof --topic MyTopic   # check type info
```

## Performance

### How fast is cyclonedds-rust?
Latency is typically in the tens of microseconds for local communication. Benchmarks are in `cyclonedds-bench/`.

### Can I use zero-copy?
Yes. Use `write_loan` for zero-copy writes and `read_loan`/`take_loan` for zero-copy reads.

### How do I reduce latency?
- Use `Reliability::BestEffort` for sensor data
- Use `Durability::Volatile` if historical data isn't needed
- Keep history depth small (`KeepLast(1)`)

## Troubleshooting

### "No publication found within timeout"
Ensure both publisher and subscriber use the same:
- Domain ID
- Topic name (case-sensitive)
- Type name

### Samples are not received
Check QoS compatibility:
- Reliable publisher + BestEffort subscriber = OK
- BestEffort publisher + Reliable subscriber = INCOMPATIBLE

### High memory usage
Limit history depth and use `take()` instead of `read()` to remove samples from the cache.
