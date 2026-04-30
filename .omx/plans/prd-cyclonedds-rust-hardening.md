# PRD: cyclonedds-rust hardening and public API parity step

## Objective
Remove local-machine build coupling and close a high-value public CycloneDDS API gap so the library is closer to being portable and production-usable.

## Requirements Summary
- Eliminate hardcoded local CycloneDDS source/build paths from `cyclonedds-sys/build.rs`.
- Default to the vendored CycloneDDS submodule when env overrides are absent.
- Build bundled CycloneDDS automatically when no external build directory is provided.
- Expose the public DDS XML QoS Provider API at the safe Rust layer.
- Allow typed topics and participants to consume provider-derived QoS directly.
- Preserve existing behavior and pass the existing test suite.

## Acceptance Criteria
- `cargo test --workspace` passes from a clean `cyclonedds-sys` rebuild.
- `cargo test --workspace --all-features` passes.
- `cargo check --workspace --examples` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes.
- `cargo build --workspace --release` passes.
- `tools/audit_ffi_coverage.py` reports 0 missing symbols for public headers.
- New tests prove `QosProvider` can load DDS XML QoS and apply it to runtime entities.

## Implementation Steps
1. Rewrite `cyclonedds-sys/build.rs` to resolve source/build directories portably and auto-build vendored CycloneDDS.
2. Add `cyclonedds::QosProvider` and `cyclonedds::QosKind` as safe wrappers over the public QoS provider API.
3. Extend `DomainParticipant` and typed `Topic<T>` constructors to accept explicit QoS.
4. Add runtime tests covering provider-loaded participant/topic/writer/reader QoS.
5. Run the full verification suite and audit remaining gaps.

## Risks and Mitigations
- Build-script portability regressions -> verify with a clean `cyclonedds-sys` rebuild.
- XML QoS parser differences -> validate via integration test against real CycloneDDS runtime.
- Scope creep toward internal API parity -> keep this step limited to public API + portability gaps.

## Verification
- Completed via cargo test/all-features/examples/clippy/release build + FFI audit.
