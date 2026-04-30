# Test Specification — CycloneDDS Rust 100% Coverage

## 1. Purpose
Define how the project will verify that every supported CycloneDDS/DDS feature is implemented correctly, using the real CycloneDDS runtime.

## 2. Test Layers

### Layer A — Unit tests
Use for deterministic local logic only:
- error mapping;
- descriptor generation;
- derive macro output shape;
- builder/state translation;
- helper utilities.

### Layer B — Same-process integration tests
Use one process with real CycloneDDS entities for:
- entity creation and teardown;
- pub/sub roundtrip;
- keyed instance lifecycle;
- listeners;
- QoS roundtrip;
- waitsets and conditions;
- statuses;
- discovery helpers when same-process behavior is sufficient.

### Layer C — Multi-process integration tests
Use separate Rust processes for:
- late joiners;
- discovery and matched endpoints;
- historical data behavior;
- domain separation;
- content-filtered topics;
- custom domain configs;
- shared-memory-sensitive behavior when meaningful.

### Layer D — Interop tests
Use external peers when needed to validate wire compatibility:
- Rust writer <-> Rust reader;
- Rust writer <-> C / reference tooling where useful;
- Rust writer <-> CycloneDDS discovery/built-in topic semantics.

### Layer E — Performance/benchmark tests
Use for:
- write throughput;
- read/take throughput;
- latency;
- loan vs copied-path behavior;
- allocation-sensitive regressions.

## 3. Test Matrix by Feature Group

## 3.1 Foundation
- workspace builds on supported host(s);
- link/load of `libddsc` succeeds;
- environment overrides work.

Acceptance:
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`

## 3.2 Core Entities
Tests:
- participant create/delete;
- topic create/delete;
- publisher/subscriber create/delete;
- writer/reader create/delete;
- parent/child/topic/publisher/subscriber lookup;
- GUID/domain/instance handle/name/type-name retrieval.

## 3.3 Types and Serialization
Tests per type family:
- primitives;
- fixed arrays;
- bounded strings;
- `String`;
- `Vec<T>`;
- nested structs;
- enums;
- keyed flat and keyed nested types.

For each type family validate:
- topic registration;
- write/read roundtrip;
- write/take roundtrip;
- derived and manual descriptor equivalence where relevant.

## 3.4 Read/Write Paths
Tests:
- `read`, `take`, `peek`;
- `read_mask`, `take_mask`, `peek_mask`;
- `read_instance`, `take_instance`, `peek_instance`;
- `read_next`, `take_next`;
- `write_ts`, `writedispose`, unregister/dispose variants;
- CDR raw APIs if exposed.

Validate:
- consumption semantics;
- status/sample-state transitions;
- timestamps where applicable.

## 3.5 Loan / Zero-Copy
Tests:
- `read_loan`;
- `take_loan`;
- automatic return on drop;
- nested loan iteration;
- no invalid memory access after loan drop;
- writer loan path if implemented.

Performance validation:
- compare loan path to copied path.

## 3.6 QoS
For each supported QoS policy:
- setter/getter roundtrip;
- application to actual DDS entities;
- behavioral test where semantics are externally observable.

Priority behavioral tests:
- reliability;
- durability / transient local;
- history;
- liveliness;
- destination order;
- presentation/coherent;
- ownership;
- batching;
- partitions;
- data representation.

## 3.7 Listeners and Status
Tests:
- all listener callbacks can be registered;
- matching callbacks fire under real conditions;
- status masks can be set/read/taken;
- status changes are observable;
- no callback lifetime misuse.

## 3.8 Waitsets and Conditions
Tests:
- waitset attach/detach;
- read condition;
- query condition;
- guard condition;
- wakeups produce expected cookies;
- entity listing from waitset.

## 3.9 Keyed Instance Management
Tests:
- register instance;
- lookup instance;
- get key from handle;
- unregister instance;
- dispose instance;
- instance-handle variants;
- multiple keys produce distinct handles.

## 3.10 Discovery and Built-in Topics
Tests:
- matched subscriptions/publications;
- participant/topic discovery;
- built-in topic payload retrieval;
- endpoint information retrieval;
- `find_topic` and `lookup_participant` behavior.

## 3.11 Filtering
Tests:
- topic filter callbacks;
- query conditions with masks;
- content-filtered topics;
- parameter updates if supported.

## 3.12 Advanced Features
Tests:
- coherent begin/end;
- suspend/resume publication;
- wait for acknowledgements;
- write flush;
- assert liveliness;
- notify readers;
- wait for historical data;
- explicit domain creation and raw config creation;
- domain deaf/mute;
- shared memory availability;
- statistics retrieval and refresh;
- type info/type obj retrieval/free.

## 4. Test Data and Utilities
The dedicated test crate (`cyclonedds-test-suite`) should provide:
- reusable message types;
- unique topic-name generation;
- process harness utilities;
- timeout helpers;
- benchmark utilities;
- config fixtures (XML/raw config) for domain and QoS provider tests.

## 5. Environment Rules
- Tests must run against a real CycloneDDS runtime.
- No mock DDS transport in integration tests.
- Multi-process tests must use unique topic names and predictable cleanup.
- Timeouts should be generous enough for CI but low enough to catch hangs.

## 6. Release-Test Gates
A feature group is complete only if:
- unit tests pass where applicable;
- integration tests pass against CycloneDDS;
- regressions are captured in the suite;
- documentation/examples compile when the feature is public.

## 7. Commands
Minimum validation command set:
- `cargo fmt --all`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets --no-deps -- -D warnings`
- `cargo test --workspace`

Feature-specific validation may add:
- `cargo test -p cyclonedds-test-suite --test <name>`
- benchmark commands to be defined in later phases.
