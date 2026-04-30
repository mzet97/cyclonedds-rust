# PRD — CycloneDDS Rust 100% Functional Coverage

## 1. Objective
Deliver a production-grade Rust binding for Eclipse CycloneDDS that provides:
- complete practical coverage of CycloneDDS/DDS features supported by the target CycloneDDS version;
- a faithful low-level FFI crate (`cyclonedds-sys`);
- a high-level safe crate (`cyclonedds`) with idiomatic Rust APIs;
- zero fake or simulated DDS behavior in runtime code;
- a real integration test suite in Rust exercising CycloneDDS itself.

## 2. Product Goal
A Rust user should be able to build real DDS systems using CycloneDDS without needing to drop to C for missing core features.

## 3. Definition of “100%”
“100%” for this project means all of the following are true for the chosen CycloneDDS version:
1. All relevant public CycloneDDS C APIs are exposed in `cyclonedds-sys`.
2. All meaningful DDS/CycloneDDS features have safe or intentionally documented unsafe Rust access paths.
3. Serialization and type support are real, not mocked.
4. Tests use the real CycloneDDS runtime and validate behavior end-to-end.
5. Performance-sensitive paths avoid unnecessary copying and allocation.
6. Public APIs are documented, lint-clean, and release-grade.

## 4. Scope

### In scope
- FFI coverage of public CycloneDDS APIs.
- Safe wrappers for entity lifecycle, publishing, subscribing, readers, writers, topics, listeners, status, waitsets, conditions, QoS, discovery, built-in topics, filtering, advanced domain APIs, statistics, type discovery, and shared-memory-aware paths.
- Real descriptor/type support for:
  - primitives;
  - fixed arrays;
  - bounded strings;
  - `String`;
  - sequences/`Vec<T>`;
  - nested structs;
  - enums;
  - keyed types including nested-key cases.
- Derive macro support for all supported type patterns.
- Test suite covering same-process and multi-process DDS scenarios.
- Benchmark and performance validation for hot paths.

### Out of scope
- Simulated DDS behavior for missing features.
- “Temporary” placeholders exposed as final public API.
- New dependencies without clear architectural value.

## 5. Users
- Rust systems engineers building distributed real-time/data-centric systems.
- Researchers and teams already using CycloneDDS in C, C++, .NET, or Python.
- Internal users of this repository who need direct Rust integration with DDS.

## 6. Success Criteria
- `cargo check --workspace` passes.
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings` passes.
- `cargo test --workspace` passes against real CycloneDDS.
- Coverage matrix marks all targeted CycloneDDS feature groups complete.
- Hot-path benchmarks demonstrate no avoidable copies on zero-copy/loan paths.
- Public APIs are documented and examples compile.

## 7. Architectural Principles
- `cyclonedds-sys` is thin and faithful.
- `cyclonedds` contains the safe, idiomatic abstraction layer.
- Unsafe code is isolated and documented with invariants.
- Ownership/lifetime rules must be explicit.
- Zero-copy is preferred when CycloneDDS supports it.
- Error surfaces must preserve real DDS failure semantics.
- Avoid hidden allocations in read/write hot paths.

## 8. Current Baseline
Current repository state already includes:
- core entities (`DomainParticipant`, `Publisher`, `Subscriber`, `Topic`, `DataWriter`, `DataReader`);
- basic descriptor-based topic registration;
- basic keyed instance support;
- partial listener/status/waitset/QoS support;
- a real initial test suite;
- recently added QoS getters, waitset entity listing, matched endpoint helpers, and stronger loan handling.

Major gaps remain in:
- complete FFI audit;
- full type system support (`String`, sequences, nested structs, enums);
- full QoS surface;
- built-in topic/discovery payload wrappers;
- filtering/content-filtered topics;
- advanced domain/statistics/type-discovery/shared-memory coverage.

## 9. Milestones

### M1 — Foundation Hardening
Stabilize workspace, lint/test hygiene, build/link behavior, API organization.

### M2 — FFI Completeness
Audit public C API and ensure raw exposure in `cyclonedds-sys`.

### M3 — Core Entity Model
Finish safe lifecycle/introspection model for all core entities.

### M4 — Type System Phase 1
Flat structs, primitives, arrays, bounded strings, keyed flat types.

### M5 — Type System Phase 2
`String`, sequences, nested structs, enums, derive completeness.

### M6 — Read/Write Hot Path
Complete read/take/peek/mask/instance/next/CDR variants.

### M7 — Loan/Zero-Copy
Complete and optimize loan APIs, writer-side loan support when available.

### M8 — QoS Complete
Expose and validate full QoS surface, getters/setters/providers.

### M9 — Listeners/Status/Waitsets
Complete callback/status/wait/condition coverage.

### M10 — Discovery/Built-in Topics
Matched endpoints, built-in topic payloads, lookup/find/discovery APIs.

### M11 — Filtering
Topic filters, query conditions, content-filtered topics.

### M12 — Advanced CycloneDDS Features
Coherent access, suspend/resume, historical data, custom domains, statistics, type discovery, shared memory, domain controls.

### M13 — Async and Ergonomics
Async APIs, examples, docs, public ergonomics cleanup.

### M14 — Final Test and Release Gate
Full feature test matrix, multi-process tests, performance evidence, release readiness.

## 10. Non-Functional Requirements

### Performance
- No unnecessary copies in reader/writer hot paths.
- Loan APIs must preserve zero-copy semantics where available.
- Avoid per-sample heap allocation in common loops.
- Listener and waitset pathways must minimize overhead.

### Safety
- No UB in descriptor, loan, callback, or entity lifecycle code.
- No resource leaks in normal or error paths.
- Thread-safety guarantees must be explicit.

### Maintainability
- Small, reviewable phases.
- Clear module ownership.
- Minimal duplication between reader/writer/status/filter code.

## 11. Release Gates
A milestone is release-ready only if:
- code compiles and lints cleanly;
- real tests for the feature set pass;
- examples for the affected surface compile;
- no known memory-safety regressions remain;
- docs reflect the actual implemented behavior.

## 12. Final Acceptance
The project can claim “100% CycloneDDS/DDS functional coverage” only when the feature matrix, test-spec, and backlog completion state all indicate no remaining targeted gaps for the chosen CycloneDDS version.
