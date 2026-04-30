# Remaining 100% Plan — cyclonedds-rust

## Goal
Reach a defensible “100% functional CycloneDDS coverage” claim for the chosen CycloneDDS build/runtime target, with real DDS behavior, no mocks, and a release-grade Rust API.

## Current baseline
Already implemented and validated with real tests:
- broad FFI public-surface coverage for the audited public headers;
- safe wrappers for participants, topics, publishers, subscribers, readers, writers, listeners, waitsets, conditions, status, QoS, statistics;
- descriptor-driven topic creation and discovery flows;
- `String`, sequences, bounded sequences, nested structs, enums, optionals, arrays, nested sequences, direct Rust ergonomic type support;
- dynamic type registration for structure / enum / union / bitmask / alias / sequence / array / string / map-builder paths;
- topic-descriptor metadata, typeinfo-driven topic creation, matched-endpoint introspection, sertype access;
- extensive real test coverage across the implemented surface.

## What still blocks a true 100% claim
1. Dynamic data/value layer is still missing.
2. `TypeInfo` / `TypeObject` wrappers are still pointer/byte oriented instead of high-level introspection APIs.
3. Built-in topic coverage is still partial; endpoint discovery is strong, but dedicated built-in topic surfaces are not complete.
4. Some advanced CycloneDDS feature groups still need closure or build-flag-specific handling.
5. Remaining invalid/edge-case dynamic-type matrices need broader coverage.
6. Release-quality documentation/examples still lag the implementation breadth.

---

# Phase A — High-level XTypes / Type metadata

## Objective
Turn current raw type metadata support into a real Rust introspection surface.

### A1. Type metadata wrapper enrichment
Deliver:
- richer `TypeInfo` / `TypeObject` APIs;
- typed metadata summaries instead of only opaque pointers/bytes;
- helper views for type-information and type-mapping payloads.

Acceptance:
- callers can inspect type metadata shape without dropping to raw pointers.
- tests validate descriptor/typeinfo/typeobject relationships on real entities.

### A2. `TypeInfo` action surface completion
Deliver:
- direct topic creation/descriptors already present, plus ergonomic helpers around them;
- explicit scope/timeout helpers;
- better behavior around unsupported/null metadata paths.

Acceptance:
- `TypeInfo` is sufficient for common discovery-driven topic recreation flows.

### A3. `TypeObject` utility layer
Deliver:
- stable wrapper utilities for retrieved type objects;
- explicit support for complete/minimal lookup flows where available.

Acceptance:
- tests can retrieve type objects from real endpoints/entities and exercise wrapper behavior.

---

# Phase B — Dynamic type surface completion

## Objective
Finish the remaining dynamic-type API surface and its invalid/edge matrices.

### B1. Dynamic member/default/index surface
Deliver:
- complete member-property ergonomics;
- explicit handling of member index insertion edge cases;
- clearer default-value story (implement if supported safely, otherwise explicitly document the limitation).

Acceptance:
- real tests cover valid and invalid combinations.

### B2. Dynamic-type invalid matrix expansion
Deliver real tests for:
- duplicate member ids;
- duplicate hash ids;
- duplicate union labels/default labels;
- invalid enum values vs bit-bound;
- invalid base-type combinations;
- invalid nested/extensibility transitions.

Acceptance:
- failure behavior is captured as real CycloneDDS runtime behavior, not guessed.

### B3. Build-flag-sensitive dynamic features
Deliver:
- explicit handling/documentation for map/shared-memory/type-discovery variations across builds;
- tests that tolerate real runtime variance where CycloneDDS behavior is build-sensitive.

Acceptance:
- feature behavior is categorized as supported / unsupported / build-dependent.

---

# Phase C — Dynamic data / dynamic value layer

## Objective
Add the biggest missing functional block: value-level APIs for dynamic types.

### C1. Dynamic value abstraction design
Deliver:
- design + first implementation for runtime-owned dynamic values/samples;
- safe ownership model for dynamic payloads;
- explicit unsafe boundary documentation.

### C2. Field access APIs
Deliver:
- get/set by member id or name for primitive fields;
- union discriminator handling;
- sequence/array/string field access where supported.

### C3. Read/write integration
Deliver:
- dynamic-type values writable/readable through CycloneDDS real paths;
- sample-level tests using dynamic values rather than only descriptor registration.

Acceptance:
- Rust users can create dynamic values, populate them, and publish/inspect them without static Rust structs.

---

# Phase D — Built-in topics and discovery completion

## Objective
Complete the built-in/discovery surface beyond matched-endpoint helpers.

### D1. Built-in topic wrappers
Deliver complete wrappers for:
- `DCPSParticipant`
- `DCPSTopic`
- `DCPSPublication`
- `DCPSSubscription`

### D2. Built-in topic reader flows
Deliver:
- real topic/reader creation flows for built-in topics when supported by runtime;
- graceful handling for unsupported/builder-specific behavior.

### D3. Discovery workflow coverage
Deliver:
- multi-participant and multi-process discovery tests;
- topic recreation and metadata matching coverage;
- stronger remote discovery scenarios.

Acceptance:
- built-in/discovery coverage is no longer partial.

---

# Phase E — Advanced CycloneDDS feature closure

## Objective
Close the remaining feature groups that affect a 100% claim.

### E1. Content/topic filtering
- content filtered topics
- query/topic filter refinements
- real tests for filter semantics

### E2. Shared memory / build-sensitive features
- explicit PSMX/shared-memory exposure policy in Rust API
- test coverage where runtime supports it

### E3. Additional advanced APIs
- any remaining uncovered public API groups that materially affect end users
- close remaining audited FFI/runtime gaps

Acceptance:
- feature matrix shows no remaining high-value functional holes for the chosen build target.

---

# Phase F — Release closure

## Objective
Make the project release-grade rather than just functionally broad.

### F1. Documentation and examples
- examples per major feature group
- docs for build-sensitive behavior
- docs for supported vs unsupported runtime combinations

### F2. Test matrix hardening
- isolate heavy tests by DDS domain/process
- document environmental requirements
- add long-form/stress suites where appropriate

### F3. 100% readiness gate
Only declare “100%” when all are true:
- high-level XTypes wrappers complete;
- dynamic value layer present;
- built-in topic coverage complete;
- advanced/build-sensitive features categorized and tested;
- public docs/examples reflect actual implementation;
- test/lint/check all green.

---

# Recommended execution order
1. Phase A — high-level `TypeInfo` / `TypeObject`
2. Phase B — dynamic-type surface completion
3. Phase C — dynamic value layer
4. Phase D — built-in topics/discovery completion
5. Phase E — advanced remaining features
6. Phase F — release closure

---

# Immediate next execution slice
The best next slice is:
- **A1/A2**: enrich `TypeInfo` / `TypeObject` wrappers and tests
- then continue into **B2** invalid/edge dynamic-type matrices

This keeps momentum on the main remaining gap area without destabilizing the already-green runtime paths.
