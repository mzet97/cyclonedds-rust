# Backlog — CycloneDDS Rust 100% Coverage Execution Plan

This backlog is the execution surface for the PRD and test-spec. Each item is intended to be small enough for implementation and verification in reviewable increments.

Status legend:
- TODO
- IN PROGRESS
- BLOCKED
- DONE

---

## Phase 1 — Foundation Hardening

### P1.1 Workspace hygiene
- Status: TODO
- Priority: P0
- Depends on: none
- Write scope:
  - `Cargo.toml`
  - `.cargo/config.toml`
  - crate `Cargo.toml` files
- Tasks:
  - normalize workspace members;
  - ensure deterministic link behavior on supported host(s);
  - document env overrides for CycloneDDS paths.
- DoD:
  - workspace builds and links cleanly.

### P1.2 API/module organization audit
- Status: TODO
- Priority: P1
- Depends on: P1.1
- Write scope:
  - `cyclonedds/src/lib.rs`
  - module files under `cyclonedds/src/`
- Tasks:
  - review public exports;
  - remove dead or duplicate surfaces;
  - document module ownership.
- DoD:
  - public API is coherent and lint-clean.

### P1.3 Unsafe boundary audit
- Status: TODO
- Priority: P0
- Depends on: P1.2
- Write scope:
  - all modules with unsafe blocks
- Tasks:
  - add invariant comments;
  - isolate unsafe helpers;
  - identify memory/lifetime risk hotspots.
- DoD:
  - unsafe blocks are explainable and localized.

---

## Phase 2 — FFI Completeness

### P2.1 Public API inventory
- Status: DONE
- Priority: P0
- Depends on: P1.1
- Write scope:
  - `cyclonedds-sys/build.rs`
  - `cyclonedds-sys/wrapper.h`
  - `cyclonedds-sys/src/lib.rs`
  - planning docs if needed
- Tasks:
  - list public CycloneDDS APIs by header group;
  - compare with generated bindings;
  - record missing or malformed items.
- DoD:
  - FFI gap matrix exists.

### P2.2 Bindings gap closure
- Status: IN PROGRESS
- Priority: P0
- Depends on: P2.1
- Write scope:
  - `cyclonedds-sys/*`
- Tasks:
  - adjust wrapper includes/blocklists;
  - expose missing functions/types/constants;
  - verify callback signatures and opaque structs.
- DoD:
  - targeted public C API is available from Rust.

### P2.3 FFI smoke validation
- Status: TODO
- Priority: P1
- Depends on: P2.2
- Write scope:
  - `cyclonedds-sys/tests/*` or integration harness
- Tasks:
  - validate symbol availability and link/load behavior.
- DoD:
  - smoke checks pass on supported environment(s).

---

## Phase 3 — Core Entity Model

### P3.1 Entity relation coverage
- Status: TODO
- Priority: P0
- Depends on: P2.2
- Write scope:
  - `cyclonedds/src/entity.rs`
  - `participant.rs`, `publisher.rs`, `subscriber.rs`, `topic.rs`, `reader.rs`, `writer.rs`
- Tasks:
  - expose complete relation/introspection APIs;
  - normalize error handling across entity lookups.
- DoD:
  - entity introspection matrix covered.

### P3.2 Lifecycle and RAII audit
- Status: TODO
- Priority: P0
- Depends on: P3.1
- Write scope:
  - same as above
- Tasks:
  - verify drop semantics;
  - ensure no invalid deletes or lifetime aliasing.
- DoD:
  - core entities pass lifecycle tests.

---

## Phase 4 — Type System Phase 1

### P4.1 Descriptor foundation cleanup
- Status: TODO
- Priority: P0
- Depends on: P3.2
- Write scope:
  - `cyclonedds/src/topic.rs`
  - optional new `descriptor.rs`
- Tasks:
  - formalize descriptor-building utilities;
  - separate descriptor internals from topic lifecycle.
- DoD:
  - descriptor code is testable and reusable.

### P4.2 Flat type coverage
- Status: TODO
- Priority: P0
- Depends on: P4.1
- Write scope:
  - `topic.rs`
  - `cyclonedds-derive/src/lib.rs`
  - tests
- Tasks:
  - primitives;
  - arrays;
  - bounded strings;
  - keyed flat structs.
- DoD:
  - real roundtrip tests pass.

---

## Phase 5 — Type System Phase 2

### P5.1 `String` support
- Status: TODO
- Priority: P0
- Depends on: P4.2
- Write scope:
  - `topic.rs`
  - `derive` crate
  - tests/examples
- Tasks:
  - define descriptor and marshalling strategy for `String`;
  - ensure ownership and cleanup are safe.
- DoD:
  - `String` roundtrip tests pass.

### P5.2 Sequence support (`Vec<T>`)
- Status: TODO
- Priority: P0
- Depends on: P5.1
- Write scope:
  - same as above
- Tasks:
  - implement sequence descriptors and marshaling;
  - validate for primitive and struct element types.
- DoD:
  - `Vec<u8>` and `Vec<i32>` tests pass.

### P5.3 Nested structs and enums
- Status: TODO
- Priority: P0
- Depends on: P5.2
- Write scope:
  - same as above
- Tasks:
  - nested descriptor composition;
  - enum encoding/decoding;
  - keyed nested path handling.
- DoD:
  - nested and enum tests pass.

### P5.4 Derive completeness
- Status: TODO
- Priority: P1
- Depends on: P5.3
- Write scope:
  - `cyclonedds-derive/src/lib.rs`
- Tasks:
  - support all implemented field patterns;
  - improve compile-time diagnostics.
- DoD:
  - derive tests cover all supported families.

---

## Phase 6 — Read/Write Hot Path

### P6.1 Read/take/peek completeness
- Status: TODO
- Priority: P0
- Depends on: P5.4
- Write scope:
  - `reader.rs`
  - tests
- Tasks:
  - complete mask/instance/next variants;
  - remove duplication in read path internals.
- DoD:
  - semantic tests pass for all variants.

### P6.2 Writer advanced variants
- Status: TODO
- Priority: P0
- Depends on: P6.1
- Write scope:
  - `writer.rs`
  - tests
- Tasks:
  - complete timestamp/dispose/unregister/flush variants;
  - validate lifecycle transitions.
- DoD:
  - writer advanced tests pass.

### P6.3 CDR raw path
- Status: TODO
- Priority: P1
- Depends on: P6.2
- Write scope:
  - `reader.rs`
  - `writer.rs`
  - possibly new raw CDR module
- Tasks:
  - expose readcdr/takecdr/writecdr/forwardcdr safely or intentionally as advanced unsafe APIs.
- DoD:
  - raw-path tests pass.

---

## Phase 7 — Loan / Zero-Copy

### P7.1 Reader-side loan completion
- Status: TODO
- Priority: P0
- Depends on: P6.1
- Write scope:
  - `sample.rs`
  - `reader.rs`
  - tests
- Tasks:
  - finalize loan ergonomics and invariants;
  - validate against multiple sample patterns.
- DoD:
  - loan stability and regression tests pass.

### P7.2 Writer-side loans / shared-memory-aware publish path
- Status: TODO
- Priority: P1
- Depends on: P7.1, P12 shared-memory work
- Write scope:
  - `writer.rs`
  - tests/benchmarks
- Tasks:
  - expose request-loan flow when available.
- DoD:
  - zero-copy publish path tested where supported.

---

## Phase 8 — QoS Complete

### P8.1 Remaining setters
- Status: TODO
- Priority: P0
- Depends on: P2.2
- Write scope:
  - `qos.rs`
- Tasks:
  - fill any remaining `qset_*` gaps.
- DoD:
  - setter matrix complete.

### P8.2 Remaining getters
- Status: TODO
- Priority: P0
- Depends on: P8.1
- Write scope:
  - `qos.rs`
- Tasks:
  - fill remaining `qget_*` coverage.
- DoD:
  - getter matrix complete.

### P8.3 Behavioral QoS tests
- Status: TODO
- Priority: P0
- Depends on: P8.2
- Write scope:
  - `cyclonedds-test-suite/tests/qos.rs`
  - config fixtures if needed
- Tasks:
  - add behavior tests for reliability, durability, liveliness, presentation, ownership, batching, partitions, data representation.
- DoD:
  - QoS feature group test matrix complete.

### P8.4 QoS provider support
- Status: TODO
- Priority: P1
- Depends on: P8.2
- Write scope:
  - `qos.rs`
  - fixtures
- DoD:
  - QoS XML/provider loading tested.

---

## Phase 9 — Listeners, Status, Waitsets

### P9.1 Listener callback completeness
- Status: TODO
- Priority: P0
- Depends on: P2.2
- Write scope:
  - `listener.rs`
  - tests
- Tasks:
  - verify all supported listener types;
  - test callback firing paths.
- DoD:
  - callback matrix complete.

### P9.2 Status API completeness
- Status: TODO
- Priority: P0
- Depends on: P9.1
- Write scope:
  - `entity.rs`
  - tests/status.rs
- DoD:
  - read/take/set/get/change workflows tested.

### P9.3 Query/read/guard condition refinement
- Status: TODO
- Priority: P1
- Depends on: P9.2
- Write scope:
  - `waitset.rs`
  - tests/advanced.rs
- DoD:
  - condition behaviors are stable and covered.

---

## Phase 10 — Discovery and Built-in Topics

### P10.1 Discovery module introduction
- Status: TODO
- Priority: P0
- Depends on: P2.2, P3.1
- Write scope:
  - new `cyclonedds/src/discovery.rs`
  - `lib.rs`
- Tasks:
  - collect discovery-related wrappers into one module.
- DoD:
  - discovery API organization is clear.

### P10.2 Matched endpoint payload wrappers
- Status: TODO
- Priority: P0
- Depends on: P10.1
- Write scope:
  - discovery module
  - tests
- Tasks:
  - safe wrappers around matched publication/subscription data;
  - free semantics handled correctly.
- DoD:
  - matched endpoint data tests pass.

### P10.3 Built-in topic support
- Status: TODO
- Priority: P1
- Depends on: P10.2
- Write scope:
  - new `builtin.rs`
  - tests/discovery.rs
- DoD:
  - built-in topic reads pass in real discovery scenarios.

---

## Phase 11 — Filtering

### P11.1 Topic filter APIs
- Status: TODO
- Priority: P1
- Depends on: P2.2, P9.3
- Write scope:
  - `topic.rs`
  - `reader.rs`
  - tests/filtering.rs
- DoD:
  - topic-filter callback tests pass.

### P11.2 Content-filtered topics
- Status: TODO
- Priority: P0
- Depends on: P11.1
- Write scope:
  - `topic.rs`
  - tests/filtering.rs
- DoD:
  - content-filtered-topic tests pass.

---

## Phase 12 — Advanced CycloneDDS Features

### P12.1 Coherent/suspend/acks/liveliness
- Status: TODO
- Priority: P0
- Depends on: P8.3, P9.3
- Write scope:
  - `publisher.rs`
  - `writer.rs`
  - tests/advanced.rs
- DoD:
  - advanced writer/publisher behavior tests pass.

### P12.2 Historical data and domain controls
- Status: TODO
- Priority: P0
- Depends on: P10.1
- Write scope:
  - `participant.rs`
  - new `domain.rs`
  - tests/domain.rs
- DoD:
  - custom domain and historical data tests pass.

### P12.3 Statistics and type discovery
- Status: TODO
- Priority: P1
- Depends on: P10.2
- Write scope:
  - new `statistics.rs`
  - new `xtypes.rs` or `typesupport.rs`
  - tests/statistics.rs
- DoD:
  - statistics/type info tests pass.

### P12.4 Shared memory support surface
- Status: TODO
- Priority: P1
- Depends on: P7.2
- Write scope:
  - `entity.rs`
  - `writer.rs`
  - `tests/shared_memory.rs`
- DoD:
  - shared-memory-capability tests pass on supported environment.

---

## Phase 13 — Async, Docs, Examples

### P13.1 Async API review
- Status: TODO
- Priority: P1
- Depends on: P9.3
- Write scope:
  - `async.rs`
  - tests/examples
- DoD:
  - async tests pass.

### P13.2 Example set completion
- Status: TODO
- Priority: P1
- Depends on: feature milestones
- Write scope:
  - `cyclonedds/examples/*`
- Example targets:
  - hello world;
  - keyed topics;
  - QoS;
  - listeners;
  - waitsets;
  - discovery;
  - filtering.
- DoD:
  - examples compile and run.

### P13.3 Rustdoc pass
- Status: TODO
- Priority: P1
- Depends on: nearly all milestones
- Write scope:
  - public modules
- DoD:
  - public API documented accurately.

---

## Phase 14 — Full Test and Release Gate

### P14.1 Multi-process harness
- Status: TODO
- Priority: P0
- Depends on: most feature milestones
- Write scope:
  - `cyclonedds-test-suite/src/*`
  - `tests/*`
- DoD:
  - cross-process tests are reproducible.

### P14.2 Feature matrix completion review
- Status: TODO
- Priority: P0
- Depends on: all previous phases
- Write scope:
  - planning docs
- Tasks:
  - review PRD/test-spec/backlog against implemented state;
  - mark final gaps.
- DoD:
  - no targeted feature gaps remain.

### P14.3 Performance release gate
- Status: TODO
- Priority: P1
- Depends on: P7, P12, P13
- Write scope:
  - benchmark harness/docs
- DoD:
  - performance evidence recorded and acceptable.

---

## Recommended Immediate Execution Order
1. P2.1 Public API inventory
2. P2.2 Bindings gap closure
3. P5.1 `String` support
4. P5.2 Sequence support
5. P5.3 Nested structs and enums
6. P8.1/P8.2 QoS closure
7. P10 discovery work
8. P11 filtering
9. P12 advanced features

This order maximizes unblock value while keeping diffs reviewable.
