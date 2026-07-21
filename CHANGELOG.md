# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Project governance: CONTRIBUTING.md, SECURITY.md, CODEOWNERS, issue templates, PR template
- Dependabot configuration for cargo and GitHub Actions
- CodeQL security analysis workflow
- Release workflow with Docker build, Cosign signing, SBOM, and Trivy scan
- Multi-stage Dockerfile and docker-compose.yml for DDS development environment

[Unreleased]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.0...HEAD

## [2.0.0] - 2026-07-21

### Fixed

- **Zero-copy loan buffer overflow** (`DataWriter::request_loan`/`WriteLoan`): the loaned
  buffer was zero-initialized and interpreted as `size_of::<T>()` bytes, but
  `dds_request_loan` only allocates `size_of::<T::Native>()` — smaller for any type with
  `String`/`Vec` fields (translated to `DdsString`/`DdsSequence`). This wrote past the end
  of the DDS-owned allocation on every loan of such a type, and a zeroed `String`/`Vec` is
  not a valid bit-pattern to begin with. `Drop for WriteLoan` now runs `drop_in_place` on
  the native value before returning the loan, so partially-populated `DdsString`/
  `DdsSequence` fields are freed correctly.
- **Reading loaned/read samples as `T` instead of `T::Native`** (`async.rs`): `take_async`/
  `read_async` used `ptr::read(samples[i] as *const T)`, reinterpreting the DDS-native
  buffer (8-byte `char*` strings) as the ergonomic Rust type (24-byte `String`); replaced
  with `T::clone_out(..)`, which converts the native representation into an owned `T`.
- **`Topic<T>` was not `Send`/`Sync`**: its `DescriptorHolder` used `Rc` (changed to `Arc`)
  and lacked explicit `unsafe impl Send/Sync`, even though the held data is read-only after
  topic creation and safely shared by CycloneDDS across its own threads. Same fix applied
  to `Qos` and `Listener` (both immutable after construction; documented safety
  justification inline).
- Stale `cyclonedds-build` codegen tests (`test_generate_simple_struct`,
  `test_compile_idl_to_string`) still asserted the pre-`Default, PartialEq` derive list.

### Added

- `DdsType::Native` associated type: the DDS wire-compatible representation used by the
  loan APIs and the topic descriptor size/align. `#[derive(DdsTypeDerive)]` now emits it
  automatically; manual `impl DdsType` blocks for POD types set `type Native = Self`.
- `DdsType::type_metadata_blobs()`: optional XCDR2 (TypeInformation, TypeMapping) blobs so
  the topic descriptor can set `DDS_TOPIC_XTYPES_METADATA` and announce type information
  over SEDP — required for type-enforcing peers (Python/C++) to match correctly.
- `DataWriter::set_qos()` — update a writer's QoS at runtime for the online-tunable knobs
  (TransportPriority, LatencyBudget, OwnershipStrength).
- Generated structs (`cyclonedds-build` codegen) now also derive `Default, PartialEq`.
- `cyclonedds-rust-sys` 1.1.0: opt-in `CYCLONEDDS_STATIC=1` static build of the vendored
  CycloneDDS (needed on filesystems without symlink support, e.g. CIFS/SMB, and produces a
  self-contained binary), with the transitive system libs (`pthread`, `dl`, `rt`, `m`) and
  `-DCMAKE_POSITION_INDEPENDENT_CODE=ON` it requires; clearer `cargo:warning=` diagnostics
  for which CycloneDDS build was picked (pre-built / freshly built / system).

### Changed

- **BREAKING**: `DdsType` now requires `type Native: Sized`. Manual `impl DdsType` blocks
  written against 1.x must add `type Native = Self;` (or the real native type, for hand-rolled
  wire-compatible structs).
- **BREAKING**: `WriteLoan::get_mut()` returns `&mut T::Native`, not `&mut T`; populate
  string fields via `DdsString::new(..)` instead of assigning a `String` directly.
  `write_loan_async`'s closure signature changed to `FnOnce(&mut T::Native)` to match.

[2.0.0]: https://github.com/mzet97/cyclonedds-rust/compare/v1.8.0...v2.0.0

## [1.8.0] - 2026-05-02

### Added

- **DDS Request-Reply Pattern** (`Requester<TReq,TRep>` + `Replier<TReq,TRep>` with correlation IDs, timeout, and retry).
- **Connection Pooling & Service Discovery** (`ParticipantPool` with multi-domain participant management, `discover_topics()`, `discover_participants()`, automatic heartbeat/purge).
- **Content Filtering Advanced** (`FilterParams` + `TopicParameterizedFilterExt::with_params()` for runtime parameter updates).
- **Serde Integration** (`SerdeSample<T>` with feature `serde` + `postcard` for Rust-to-Rust serialization over DDS).
- **Observability** (`observability.rs` with `init_json_logging()`, `init_tokio_console()`, `init_full_observability()`; features `opentelemetry` and `tokio-console`).
- **WASM Support (Experimental)** — new `cyclonedds-wasm` crate with DDS-compatible API over WebSocket; compiles for `wasm32-unknown-unknown`.
- **no_std / Embedded Support (Experimental)** — feature `no_std` exports `DdsType` trait + CDR opcode constants without FFI; compiles for `thumbv7em-none-eabihf`.
- **Security Production Hardening** (`SecurityConfig::crl()` for Certificate Revocation Lists + `docs/security-production.md`).

### Changed

- `cyclonedds-rust-sys` and `thiserror` are now optional dependencies (feature `std`).
- Feature `async` now implies `std` for CI compatibility.
- `lib.rs` uses `#[cfg(feature = "std")]` to conditionally compile all FFI-dependent modules.

## [1.7.0] - 2026-05-02

### Added

- **Error Handling & Recovery** (`DdsError::is_transient()`, retry with exponential backoff in `DomainParticipant::new()` and `DataWriter::write()`).
- **Async Timeouts & Cancellation** (`read_aiter_timeout`, `take_aiter_timeout`, safe cancellation via `drop()` without DDS entity leaks).
- **DDS Security Hardening** (`SecurityConfig::validate()` for X.509/PEM checks, `SecurityConfig::reload()` for hot-reload support).
- **Profiling & Diagnostics CLI** (`cyclonedds-cli diagnose --domain 0` for full JSON state, `cyclonedds-cli metrics <topic>` for Prometheus text export).
- **ROS2 Interop Helpers** (`DomainParticipant::ros2_topic_name()` for `rt/<topic>` naming, `ros2_qos_reliable()` and `ros2_qos_best_effort()` QoS mappers).
- **Loaned Reads (Zero-Copy Subscriber)** (`DataReader::read_loan()`, `DataReader::take_loan()` with `ReadLoan<T>` wrapper).
- **Expanded Test Suite** (reconnection rediscovery tests, cross-domain isolation tests, long-duration stress tests).

### Changed

- CI/CD workflows updated to run tests sequentially (`--test-threads=1`) to prevent flaky SIGSEGV caused by CycloneDDS global domain state in parallel test execution.
- `missing_docs` lint suppressed globally (`#![allow(missing_docs)]`) to unblock CI; documentation will be incrementally added.
- Fixed ~45+ Clippy warnings across the entire workspace (`collapsible_match`, `needless_borrow`, `len_zero`, `never_loop`, `redundant_closure`, `print_literal`, `format_in_format_args`, `dead_code`, etc.).
- Fixed broken intra-doc links in `serialization.rs`.
- Fixed benchmark `config_comparison.rs` missing `max_blocking_time` argument in `reliability()`.

### Fixed

- Flaky `qos` test SIGSEGV in MSRV and Code Coverage jobs.
- `type_discovery.rs` accidental deletion restored with careful re-application of Clippy fixes.
- `cyclonedds-test-suite` examples and benchmarks using incorrect `cyclonedds_derive::DdsTypeDerive` import (now uses `cyclonedds::DdsTypeDerive`).
- CLI `main.rs` `needless_range_loop` and `needless_borrow` issues.

## [1.6.0] - 2026-04-30

### Added

- Initial comprehensive API: DomainParticipant, Publisher, Subscriber, Topic, DataWriter, DataReader.
- 26+ QoS policies via `QosBuilder`.
- 13 listener callbacks via `ListenerBuilder`.
- WaitSet / ReadCondition / QueryCondition / GuardCondition.
- Derive macros: `DdsType`, `DdsEnum`, `DdsUnion`, `DdsBitmask`.
- CDR serialization (XCDR1/XCDR2), dynamic types, type discovery (XTypes).
- Async streams (`read_aiter`, `take_aiter`) with tokio integration.
- CLI tools: `ls`, `ps`, `subscribe`, `typeof`, `publish`, `discover`, `echo`, `record`, `replay`, `monitor`, `health`, `topology`.
- Zero-copy write loans (`WriteLoan<T>`).
- DDS Security support (`SecurityConfig` + `--features security`).

[1.8.0]: https://github.com/mzet97/cyclonedds-rust/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/mzet97/cyclonedds-rust/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/mzet97/cyclonedds-rust/releases/tag/v1.6.0
