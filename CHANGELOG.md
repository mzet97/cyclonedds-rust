# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-04-30

### Added
- Full v1.0 release with stable API.
- All crates published to crates.io.
- CI/CD with GitHub Actions.
- Cross-process integration tests.
- Zero-copy write loan tests and documentation.
- Latency benchmark example.

### Changed
- MSRV raised to Rust 1.85.
- `cyclonedds-src` published as separate crate.
- `cyclonedds-rust-sys` uses `cyclonedds_src::source_dir()` for bundled source.

## [Unreleased]

### Added

- Crate `cyclonedds-src` bundling Eclipse CycloneDDS C source for out-of-the-box builds without system `libddsc`.
- `rust-version = "1.85"` (MSRV) declared in workspace root and propagated to all crates.
- Prebuilt FFI bindings used unconditionally in `cyclonedds-rust-sys`; removes runtime dependency on `clang`/`bindgen` for end users.

### Changed

- `cyclonedds-rust-sys/build.rs` now resolves CycloneDDS source from `cyclonedds-src` crate first, then workspace `vendor/`, then system library.
- `Cargo.lock` kept at lockfile version 3; this also documents the failed Rust 1.70 validation path before the MSRV was raised to Rust 1.85.
- `cyclonedds-rust-sys/build.rs` now resolves bundled source via `cyclonedds_src::source_dir()`, which also works when the sys crate is built from a published package rather than from the workspace layout.
- Added `internal-ops` feature to `cyclonedds-rust-sys` to silence unexpected cfg warnings.
- Removed unused imports in `cyclonedds-cli` and `cyclonedds-build`.
- Cleaned up redundant `version` keys in workspace dependency declarations.
- Created `scripts/publish.sh` for automated sequential crate publishing.
- **Fase 3:** DDS Security explicitamente documentado como non-goal para v1.0.
- WriteLoan testado com `cyclonedds-test-suite/tests/write_loan.rs`.
- CLI documentado com limites honestos no README.
- Benchmark de latência criado em `cyclonedds-test-suite/examples/bench_latency.rs`.
- Added GitHub Actions CI: `ci.yml` (Linux/Windows/macOS), `msrv.yml`, `clippy.yml`, `doc.yml`.
- Added cross-process pub/sub integration test via `cyclonedds-test-suite/tests/interop.rs`.
- Added `#![warn(missing_docs)]` to `cyclonedds` crate; CI allows missing_docs temporarily while full docs are in progress.
- Documented core types: `DomainParticipant`, `DataReader`, `DataWriter`, `UntypedTopic`.

### Fixed

- `cyclonedds-test-suite` now aliases `cyclonedds-rust-sys` as `cyclonedds_sys`, matching the integration tests that reference raw DDS status constants.

### Validation

- `cargo build -p cyclonedds-rust-sys` passed in a WSL root copy using bundled `cyclonedds-src`.
- `cargo test --workspace --features async` passed after fixing the test-suite sys dependency alias.
- MSRV validation with Rust 1.70.0 showed the previous declaration was aspirational: current dependency resolution selects crates using Rust 2024 edition. The documented MSRV is now raised to Rust 1.85 so the release gate can be verified against the actual dependency graph.
- `cargo +1.85.0 build --workspace --all-features --locked` and `cargo +1.85.0 test --workspace --features async --locked` passed in a WSL copy.
- A temporary external consumer project built successfully against `cyclonedds` by path, exercising bundled `cyclonedds-src` and `cyclonedds-rust-sys` outside the repository workspace.

## [0.1.0] - 2025-04-??

### Added

- Initial release of `cyclonedds-rust` workspace.
- Safe Rust wrapper (`cyclonedds`) around Eclipse CycloneDDS.
- Complete DDS entity model: DomainParticipant, Publisher, Subscriber, Topic, DataWriter, DataReader.
- 26+ QoS policies via type-safe `QosBuilder`.
- 13 listener callbacks via `ListenerBuilder`.
- WaitSet / ReadCondition / QueryCondition / GuardCondition support.
- Derive macros: `DdsType`, `DdsEnum`, `DdsUnion`, `DdsBitmask`.
- CDR serialization (XCDR1/XCDR2), dynamic types, type discovery (XTypes).
- Async streams (`read_aiter`, `take_aiter`) with tokio integration.
- CLI tools: `ls`, `ps`, `subscribe`, `typeof`, `publish`.
- Build helpers (`cyclonedds-build`, `cyclonedds-idlc`) for IDL-to-Rust code generation.

[Unreleased]: https://github.com/mzet97/cyclonedds-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/mzet97/cyclonedds-rust/releases/tag/v0.1.0
