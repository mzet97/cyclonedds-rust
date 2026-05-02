# Benchmarks

The `cyclonedds-bench` crate contains Criterion-based benchmarks for measuring DDS performance.

## Running benchmarks

```bash
# All benchmarks
cargo bench -p cyclonedds-bench

# Specific benchmark
cargo bench -p cyclonedds-bench --bench latency
cargo bench -p cyclonedds-bench --bench throughput
cargo bench -p cyclonedds-bench --bench cdr
cargo bench -p cyclonedds-bench --bench ipc_comparison

# Convenience script
cd scripts && bash bench_compare.sh
```

## Available benchmarks

### `latency`

Measures round-trip latency for different payload sizes:

| Payload | Description |
|---------|-------------|
| 64 B    | Small control message |
| 1 KB    | Typical telemetry |
| 16 KB   | Large payload (e.g., image chunk) |

Results are reported as time per round-trip (write + read).

### `throughput`

Measures messages-per-second for 1 KB payloads with different batch sizes.

### `cdr`

Measures CDR serialization/deserialization performance for various types.

### `ipc_comparison`

Compares DDS round-trip latency against `std::sync::mpsc` channel latency.

## Benchmark report

After running, open the HTML report:

```bash
# Linux / macOS
open target/criterion/report/index.html

# Windows
start target/criterion/report/index.html
```

## Cross-implementation comparison (FastDDS / OpenDDS)

Full cross-implementation benchmarks require external C++ harnesses.

### Planned harness structure

```
cyclonedds-bench/external/
  fastdds/
    CMakeLists.txt
    latency_pub.cpp
    latency_sub.cpp
  opendds/
    CMakeLists.txt
    latency_pub.cpp
    latency_sub.cpp
```

### Metrics to collect

| Metric | Description |
|--------|-------------|
| p50 latency | Median round-trip time |
| p99 latency | 99th percentile round-trip time |
| p999 latency | 99.9th percentile round-trip time |
| Throughput | Messages per second |
| CPU usage | % CPU during test |

### Running a comparison

1. Build the external harnesses (see `external/README.md`)
2. Start the subscriber harness for each implementation
3. Run the publisher harness and collect results
4. Run `scripts/bench_compare.sh` to collect cyclonedds-rust results
5. Compare the output files manually or with a plotting script

## Configuration comparison

You can also compare different CycloneDDS configurations:

```bash
# SHM (Iceoryx) transport
CYCLONEDDS_URI=file://cyclonedds-shm.xml cargo bench -p cyclonedds-bench --bench latency

# UDP only
CYCLONEDDS_URI=file://cyclonedds-udp.xml cargo bench -p cyclonedds-bench --bench latency
```

## Notes

- Benchmarks run in-process by default (publisher and subscriber in the same process).
- For true inter-process measurements, use separate processes or the external harness approach.
- Results vary significantly based on hardware, OS scheduler, and network configuration.
- Always run benchmarks on a quiet machine with CPU frequency scaling disabled for reproducible results.
