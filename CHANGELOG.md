# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.5.0] - 2026-05-01

### Added

- **CDR Performance**: `CdrSerializer::serialize_to_buffer()` and
  `CdrSerializer::serialize_key_to_buffer()` for zero-allocation serialization
  into pre-allocated buffers.
- **CLI v1.5**: `monitor` command for real-time throughput statistics.
- **CLI v1.5**: `health` command for checking publisher/subscriber presence
  on one or more topics.
- **CLI v1.5**: `topology` command for generating Graphviz DOT graphs of
  discovered DDS entities.
- **Benchmarks**: `cyclonedds-bench/benches/ipc_comparison.rs` comparing
  DDS round-trip latency against std channels.
- **Tutorial**: `docs/tutorial.md` step-by-step guide for first DDS application.
- **FAQ**: `docs/faq.md` with common questions and troubleshooting tips.
- **Admin API**: `DomainParticipant::discovered_participants()`,
  `discovered_publications()`, `discovered_subscriptions()`, `discovered_topics()`
  for runtime introspection.
- **Stress Tests**: `cyclonedds-test-suite/tests/stress.rs` with 100K message
  throughput validation.
- **CI Coverage**: `.github/workflows/coverage.yml` with `cargo-llvm-cov`
  and Codecov upload.

### Changed

- Workspace version bumped to `1.5.0`.

## [1.4.0] - 2026-05-01

### Added

- **DDS Security Tests**: `cyclonedds-test-suite/tests/security.rs` with integration
  tests for `SecurityConfig` builder, QoS property application, and graceful handling
  of invalid certificate paths.
- **QoS Profiles**: `QosProvider::from_xml_with_profile()` and
  `QosProvider::from_file_with_profile()` for loading named profiles from XML.
  Example file added at `examples/qos_profiles.xml`.
- **Async Batch Timeouts**: `read_aiter_batch_timeout(max_samples, timeout_ns)` and
  `take_aiter_batch_timeout(max_samples, timeout_ns)` for combined back-pressure
  and cancellation control.
- **CLI v1.4**: `echo` command for loopback debugging (subscribe + republish).
- **CLI v1.4**: `record <file>` command for recording samples to JSON.
- **CLI v1.4**: `replay <file>` command for replaying recorded JSON samples.
- **CLI v1.4**: `subscribe --filter "field > 10"` for simple numeric field filtering.
- **IDL Compiler Tests**: Added tests for nested structs, cross-module type references
  (`Geometry::Point`), and array typedefs.
- **Observability**: `examples/metrics.rs` demonstrating `Statistics` API usage
  for writer and participant metrics.
- **SHM Transport**: `examples/shm_pub.rs` and `examples/shm_sub.rs` demonstrating
  Iceoryx/PSMX shared-memory transport with large messages.

### Changed

- Workspace version bumped to `1.4.0`.

## [1.3.0] - 2026-05-01

### Added

- **CLI v1.3**: `discover` command for listing discovered types on a topic with metadata
  (size, align, key count).
- **CLI v1.3**: `subscribe --json` flag for outputting received samples as JSON.
- **CLI v1.3**: `publish --rate <hz>` flag for publishing at a fixed frequency
  (overrides count and delay).
- **CDR Benchmarks**: New `cyclonedds-bench/benches/cdr.rs` with Criterion benchmarks
  for serialization, deserialization, and round-trip of simple and complex types.
- **ROS2 Integration**: `examples/ros2_turtlesim.rs` demonstrates publishing
  `geometry_msgs/Twist` messages to a ROS2 `turtlesim` node.
- **ROS2 Documentation**: `docs/ros2-integration.md` with topic naming, QoS mappings,
  common message types, and troubleshooting.
- **DDS Security Examples**: `examples/security_pub.rs` and `examples/security_sub.rs`
  with complete `SecurityConfig` usage.
- **DDS Security Docs**: `docs/security-guide.md` with certificate generation and setup.
- **Async Timeouts**: `read_aiter_timeout(timeout_ns)` and `take_aiter_timeout(timeout_ns)`
  for async streams with configurable timeouts.

### Changed

- Workspace version bumped to `1.3.0`.

## [1.2.0] - 2026-05-01

### Added

- **DDS Security**: `SecurityConfig` builder for configuring DDS Security via QoS properties.
  Supports identity certificates, Governance/Permissions XML, and plugin selection.
  Enable with `cargo build --features security` (requires OpenSSL).
- **Iceoryx/PSMX**: `QosBuilder::enable_iceoryx()` convenience method for shared-memory transport.
- **Async batching**: `read_aiter_batch(max_samples)` and `take_aiter_batch(max_samples)` on
  `DataReader` for back-pressure control in async streams.
- **QoS Profiles**: `QosProvider::from_xml()` alias and convenience getters:
  `get_participant_qos`, `get_publisher_qos`, `get_subscriber_qos`, `get_topic_qos`,
  `get_reader_qos`, `get_writer_qos`.
- **`[package.metadata.docs.rs]`**: Added to all publishable crates with `all-features = true`.

### Changed

- Workspace version bumped to `1.2.0`.

## [1.1.0] - 2026-05-01

### Added

- **CLI v1.1**: `publish --json` flag for publishing structured messages from JSON payloads.
  Supports primitives, structs, arrays, sequences, and enums via DynamicData.
- **CLI v1.1**: `typeof` command now displays IDL-like representation with XTypes metadata:
  keys, extensibility (`@final`/`@appendable`/`@mutable`), annotations (`@key`, `@optional`,
  `@must_understand`, `@external`), enum literals, union cases, and bitmask positions.
- **Benchmarks**: New `cyclonedds-bench` crate with Criterion benchmarks for latency
  (round-trip 64B, 1KB, 16KB) and throughput (msg/s with variable batch sizes).
- **Cargo plugin**: New `cargo-cyclonedds` crate providing `cargo cyclonedds generate <idl>`
  command with `--output-dir`, `--cyclonedds-home`, `--module-name`, and `--no-idlc` flags.
- **CI/CD**: GitHub Actions workflows passing on Ubuntu, Windows, and macOS.
  Windows CI fixed by disabling DDS Security (`-DENABLE_SECURITY=OFF`) and SSL
  (`-DENABLE_SSL=OFF`) in bundled CycloneDDS build.

### Fixed

- **Docs.rs**: Resolved ~820 `missing_docs` warnings. Internal modules now use
  `#[allow(missing_docs)]` to suppress bindgen/FFI noise without reducing API coverage.
- **Docs.rs**: Fixed broken intra-doc link in `status.rs` (`crate::error::check`).
- **Docs.rs**: Added `#[allow(rustdoc::broken_intra_doc_links)]` and
  `#[allow(rustdoc::invalid_html_tags)]` to `cyclonedds-rust-sys` generated bindings.
- **Doctest**: Fixed broken doctest in `cyclonedds/src/lib.rs` to use `DdsTypeDerive` struct.

### Changed

- Bumped workspace version to `1.1.0`.
- `cyclonedds-rust-sys` bumped to `1.0.2` (CI fixes).

## [1.0.0] - 2026-04-30

### Added

- Initial stable release.
- Safe Rust bindings for Eclipse CycloneDDS core entities.
- 26+ QoS policies via `QosBuilder`.
- 13 listener callbacks via `ListenerBuilder`.
- WaitSet / Conditions for event-driven architectures.
- Derive macros: `DdsType`, `DdsEnum`, `DdsUnion`, `DdsBitmask`.
- CDR serialization (XCDR1/XCDR2), dynamic types, type discovery (XTypes).
- Async streams (`read_aiter`, `take_aiter`) with tokio integration.
- CLI tools: `ls`, `ps`, `subscribe`, `typeof`, `publish`, `perf`.
- IDL compilation support via `cyclonedds-build` and `cyclonedds-idlc`.
