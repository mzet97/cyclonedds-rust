# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
