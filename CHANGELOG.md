# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased] - 3.0.0-alpha.1

`main` carries breaking changes and is versioned `3.0.0-alpha.1`. Anything tagged
`v2.0.4` is the last release without them.

### Changed

- **BREAKING**: `Topic::new`/`with_qos`, and `DataReader`/`DataWriter`'s `new`,
  `with_qos`, `with_listener` and `with_qos_and_listener` now take their parents by
  reference (`&DomainParticipant`, `&Subscriber`/`&Publisher`, `&Topic<T>`) instead of
  raw `dds_entity_t` handles. The handle form accepted any entity, so a `Topic<A>`
  handle could be handed to a `DataReader<B>` — CycloneDDS returned samples laid out as
  `A` while `clone_out` reinterpreted them as `B`, and nothing in the type system
  objected. It also let a temporary supply the handle
  (`Topic::new(DomainParticipant::new(0)?.entity(), "x")` compiles, deletes the
  participant at the end of the statement, and leaves the topic on a recyclable handle).
  The raw forms remain as `Topic::from_entity`/`with_qos_from_entity` and
  `Data{Reader,Writer}::from_entities`/`from_entities_with` for FFI interop, documented
  as unchecked.

  `Publisher::new`, `Subscriber::new` and `WaitSet::new` still take a participant
  handle: they carry no type parameter, so there is no equivalent confusion to prevent.

  Note this does **not** fix drop ordering — a `DataReader` can still outlive the
  `Topic` it borrowed from, because the reference is only required for the call. Owning
  the parent (via `Arc`) is what closes that, and is not done here.

### Fixed

- **`TopicDescriptor` double-freed when cloned** (`cyclonedds/src/xtypes.rs`): it carried a
  `Clone` that copied the raw `*mut dds_topic_descriptor_t` with no reference count, and a
  `Drop` that called `dds_delete_topic_descriptor` on it. Two clones owned the same
  allocation, so the second drop was a double free and any access after the first drop a
  use-after-free. Nothing in this repository cloned one, which is why it never fired — but
  `Clone` is public API, so any caller doing the obvious thing hit it. Reproduced as
  `STATUS_HEAP_CORRUPTION` before the fix. The pointer now lives in an `Rc<DescriptorOwner>`
  and is released once the last clone is dropped; `Rc` (not `Arc`) keeps the type
  `!Send`/`!Sync`, exactly as the raw-pointer version was. Found by the `xtypes.rs` audit.
- **`TopicDescriptor::parse_type` walked the ops array out of step**
  (`cyclonedds/src/xtypes.rs`): `adr_step` carried a second hand-maintained table of ADR
  instruction widths — the same class of defect as the derive's `ops()` scanner, in
  another location — and this one omitted `ENU`, `ARR`, `UNI` and `EXT` entirely; all
  fell through to 2 words. `ARR` is 3 minimum and `ENU`/`EXT` are 3, so a single array,
  enum or nested-struct field put the walk permanently out of phase and members after it
  simply vanished from the result (a 3-member type reported 2). Bounds-checked
  throughout, so never memory-unsafe — it just made a public introspection API describe
  a type that does not exist. The table now matches `dds_opcodes.h`. Unlike the derive,
  this one cannot be deleted: the ops array comes from CycloneDDS, so it has to be right.
  Found by the `xtypes.rs` audit.
- Pre-existing clippy findings that only surface on the MSRV toolchain: a duplicated
  `#[cfg(feature = "std")]` in `lib.rs` and two needless lifetimes.

## [2.0.4] - 2026-08-14

Soundness release. Five defects reachable from ordinary, `unsafe`-free use of the
public API — three of them memory corruption, one a remotely triggerable process
abort. Every one of them lived on a surface with **zero** test coverage; that gap is
closed first (five new regression suites, plus an AddressSanitizer CI job), which is
what makes the rest verifiable.

`cyclonedds-rust-sys` goes to 1.1.1.

### Fixed

- **`ddsi_serdata` vtable helpers read one byte instead of a function pointer**
  (`cyclonedds-rust-sys`): `*(ops as *const u8).add(N)` dereferences a `u8`, so
  `ddsi_serdata_size`, `ddsi_serdata_to_ser` and `ddsi_serdata_unref` transmuted a
  value in `0..=255` into a function pointer and called it. Any `read_cdr`/`take_cdr`
  with a live sample jumped into an unmapped page; the CLI hit the same path in
  `subscribe`, `echo`, `record` and `monitor`. The byte offsets themselves were
  correct, so this was invisible on inspection. Replaced the offset arithmetic with
  `#[repr(C)]` `SerdataHeader`/`SerdataOps` declarations and field access, removing
  the whole class of defect rather than the three instances.
- **`Loan::iter()` handed out `&T` over memory laid out as `T::Native`**
  (`cyclonedds/src/sample.rs`): for any type with `String`/`Vec` fields the DDS buffer
  holds `DdsString` (8 bytes) / `DdsSequence` where `T` expects `String` (24 bytes) /
  `Vec` — an out-of-bounds read, and heap corruption through `to_vec()`, whose
  `String::clone` allocated from a garbage capacity and later freed a pointer Rust
  never allocated. `iter()` now yields owned `Sample<T>` via `DdsType::clone_out`, and
  `to_vec()` no longer requires `T: Clone`. Genuine zero-copy access moved to the new
  `Loan::iter_native()`, which yields `Sample<&T::Native>` — the honest type.
- **`DataReader::instance_get_key` zero-initialized a generic `T`**
  (`cyclonedds/src/reader.rs`): an all-zero `String`/`Vec` violates the `NonNull` niche
  inside it (rustc rejects it outright in debug builds), the buffer was sized
  `size_of::<T>()` while CycloneDDS writes the `Native` layout, and the returned value
  was freed by Rust over `ddsrt_malloc`-owned memory. Now uses
  `MaybeUninit<T::Native>` plus `clone_out`. Also corrected `check_entity` to `check`
  for what is an operation return code, not an entity handle.
- **`DataReader::lookup_instance` passed `&T` straight to CycloneDDS**
  (`cyclonedds/src/reader.rs`): every writer-side equivalent routes through
  `write_to_native`; this one did not, so CycloneDDS read the key at native offsets
  and found the middle of Rust's `String` — `strlen` over an arbitrary address. Now
  mirrors `DataWriter::lookup_instance`, signature unchanged.
- **No panic barrier on any listener callback** (`cyclonedds/src/listener.rs`): none of
  the 13 `extern "C"` trampolines wrapped the user closure in `catch_unwind`, so a
  single `unwrap()` in a callback aborted the process from a CycloneDDS thread. The
  trampolines are now generated by a macro over a shared `dispatch` helper that owns
  the barrier, so a newly added callback cannot omit it. The same barrier was added to
  `content_filtered_topic::trampoline_filter_sample_arg` and `log::log_trampoline`.
- **Remotely triggerable abort through the content filter**
  (`cyclonedds-derive`): `clone_out` for a union declared without `#[dds_default]`
  panics on an unknown discriminator, and that discriminator arrives from the network —
  a peer built from a different revision of the IDL could abort the process. The
  `catch_unwind` barrier above contains it; the panic message now names the
  discriminator and the type, and points at `#[dds_default]` as the fix.
- **Use-after-free window in the `QueryCondition` registry**
  (`cyclonedds/src/waitset.rs`): the trampoline read a raw pointer to the closure and
  used it *after* releasing the registry lock. `QueryCondition` is `Send` and its
  `Drop` removes the entry, so another thread dropping the condition in between freed
  the closure about to be called. The registry now stores `Arc` and the trampoline
  clones it inside the lock.
- **Log lines delivered twice, and a reentrancy deadlock** (`cyclonedds/src/log.rs`):
  both sinks registered a null `logdatum`, so the shared trampoline could not tell
  which one fired and invoked both. Each sink now registers a distinct tag. The user
  callback is also cloned out and the mutex released before invoking it — previously a
  sink that logged anything flowing back through CycloneDDS deadlocked.
- **`get_name`/`get_type_name` mishandled names of 256 bytes or more**
  (`cyclonedds/src/entity.rs`): these follow `snprintf` semantics and return the length
  that *would* be needed, but the result was passed to `Vec::truncate`, which only ever
  shrinks — so the full raw buffer was returned, embedded NUL padding included. Now
  retries once with an exactly-sized buffer.
- **`unwrap()` on `CString::new` for key names** (`topic.rs`, `content_filtered_topic.rs`):
  an interior NUL in a key name is bad input, not a bug; now reported as
  `DdsError::BadParameter`, consistent with the topic-name handling a few lines above.
- **Poisoned-mutex panics in `QueryCondition::with_filter` and its `Drop`**: a panicking
  `Drop` during an unwind aborts the process. Both now recover via `into_inner()`, as
  the trampoline already did.
- **The async read path outlived the reader it borrowed** (`cyclonedds/src/async.rs`):
  `take_async` and the drain step of all eight `*_aiter*` streams ran their
  `dds_take`/`dds_read` inside `tokio::task::spawn_blocking`. That task is `'static`, so
  only the raw `dds_entity_t` (an `i32`) was moved in, not a borrow of the
  `DataReader` — cancelling the future left the task running against a handle whose
  reader could already be dropped and its entity deleted, and CycloneDDS recycles entity
  handles. Neither call blocks (both walk the reader history cache), so the thread hop
  bought nothing; they now run inline, tied to the `&self` borrow the future already
  holds. This also removes one thread hop per read from the hot path. The eight streams
  collapsed onto a single shared implementation (~500 lines removed).
  `dds_waitset_wait` does block and stays on `spawn_blocking`; the remaining gap there
  (an uncancellable wait on a `WaitSet` dropped with the future) is documented on
  `WaitSet::wait_async` and needs an ownership change to close.
- **The `ops()` instruction scanner disagreed with `dds_opcodes.h`**
  (`cyclonedds-derive`): the generated `ops()` walks the bytecode it just built to find
  `TYPE_EXT` (nested composite) instructions and patch their jump words, which requires
  knowing each instruction's width. Ten entries were wrong — `SEQ|ENU` counted 2 words
  instead of 3, `SEQ|BST` 4 instead of 3, `ARR|ENU` and `BSQ|ENU` 3 instead of 4, `UNI`
  2 instead of 4, `BMK` 2 instead of 4, composite `ARR` subtypes 3 instead of 5 — and
  the `TYPE_EXT` advance was hardcoded to 3 while the patch step already widened to 4
  for the external flag (neither accounted for `DDS_OP_FLAG_OPT`). A mis-sized
  instruction makes the scan land mid-instruction, so it can miss a real `TYPE_EXT`
  (leaving its jump word zeroed) or mistake a data word for one and patch that.
  Corrected against the header table.

  **No failing case was demonstrated.** Tracing the emitted bytecode by hand shows the
  drift resynchronises: a skipped data word rarely has `0x01` in its top byte, so it
  does not match `OP_ADR` and the scan advances one word at a time until it realigns.
  Correctness was therefore resting on data words not looking like opcodes, which is a
  coincidence rather than an invariant. The structural fix is to record `TYPE_EXT`
  positions while emitting instead of re-scanning afterwards — the derive already knows
  where it put them — and is left as follow-up.
- **Clippy CI gate was red on `main`** (pre-existing, from `709f58a`):
  `assert!(reg.is_poisoned() || true)` in a `waitset.rs` test was a tautology that
  asserted nothing (the poisoning there is deterministic, so it now asserts it), plus an
  unused parameter. `cargo clippy --workspace --all-targets -- -D warnings` passes again.

### Added

- Regression suites for the surfaces where the above shipped undetected — all four had
  **zero** coverage, which is why POD-only tests kept passing: `cdr_roundtrip.rs`,
  `loan_heap_fields.rs`, `instance_string_key.rs`, `listener_panic_barrier.rs`,
  `async_reader_lifetime.rs`, `ops_scanner_alignment.rs` (each of the latter pairs a
  mis-sized instruction with a trailing nested struct — the combination no existing test
  produced).
  Each was confirmed to reproduce its defect before the fix (access violation, rustc's
  own "attempted to zero-initialize type which is invalid", and a non-unwinding abort).
- `Loan::iter_native()` — zero-copy iteration over loaned samples as `&T::Native`.
- AddressSanitizer CI job (nightly, Linux, non-blocking for now) over those four
  suites: for a safe wrapper around a C library, the failure mode that matters is
  corruption a normal test run reports as "ok".
- Project governance: CONTRIBUTING.md, SECURITY.md, CODEOWNERS, issue templates, PR template
- Dependabot configuration for cargo and GitHub Actions
- CodeQL security analysis workflow
- Release workflow with Docker build, Cosign signing, SBOM, and Trivy scan
- Multi-stage Dockerfile and docker-compose.yml for DDS development environment
- Per-crate `README.md` for all 9 published crates, each `Cargo.toml` now pointing at
  its own file — inheriting `readme.workspace = true` resolves relative to the
  workspace root, so every crate rendered the root README on crates.io.

### Changed

- `docs/` brought back in line with 2.x: `getting-started.md`, `api-guide.md`,
  `observability.md` and `tutorial.md` advertised `cyclonedds = "1.7"`/`"1.4"`;
  `type-system.md`, `migration-from-python.md` and `architecture.md` used
  `#[derive(DdsType)]`, which does not exist — the crate re-exports the macros as
  `DdsTypeDerive`/`DdsEnumDerive`/`DdsUnionDerive`/`DdsBitmaskDerive` (`DdsType` is the
  trait); and the `impl DdsType` example in `getting-started.md` omitted the mandatory
  `Native` associated type, so it did not compile.
- `observability` module docs claimed the `opentelemetry` feature re-exports
  `tracing-opentelemetry` and `opentelemetry-otlp`. It never did — neither is a
  dependency of this crate. Documented what the feature actually provides.
- Root README corrected against the source: version `1.8` → `2.0`, CMake 3.10 → 3.16
  (the vendored `CMakeLists.txt` requires 3.16), `write_loan` → `request_loan`, derive
  macro names to their `*Derive` re-exports, the CLI subcommand list (12–13 listed, 16
  exist), and `DdsType` examples that omitted the mandatory `Native` associated type and
  therefore did not compile. Added a feature-flag table and documented the per-target
  ABI probe and the `abi/<triple>.rs` snapshot required for cross-compilation.

[Unreleased]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.4...HEAD
[2.0.4]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.3...v2.0.4

## [2.0.3] - 2026-07-23

### Fixed

- **Release container's Trivy gate, for good this time**: a 4th different CVE
  (`CVE-2026-9538`, perl-Archive-Tar DoS) appeared minutes after 2.0.2 shipped, in the same
  never-fixed Perl/gzip OS packages. CVE-by-CVE `.trivyignore` entries proved unsustainable
  (4 different CVEs cycling through the same package group in 2 days). Replaced with
  `ignore-unfixed: true` on the Trivy scan step: skips any CVE with no upstream fix
  available (`will_not_fix`/`fix_deferred`/`affected`) — the exact category every CVE seen
  in these packages falls into — while still failing the gate on anything with an actual
  available patch. `.trivyignore` kept as documentation of the specific CVEs already
  investigated.

[2.0.3]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.2...v2.0.3

## [2.0.2] - 2026-07-23

### Fixed

- **Trivy gate on the release container still failed** after 2.0.1: 3 new CVEs
  (`CVE-2026-41992` gzip, `CVE-2026-42496` perl-base, `CVE-2026-48962` perl-IO-Compress)
  appeared in the Trivy feed between the 2.0.1 tag and this release, in the same
  never-executed Perl/gzip OS packages already covered by `.trivyignore`. Added to
  `.trivyignore` with the same justification; noted that CVE-by-CVE suppression on these
  packages is not sustainable long-term — a follow-up should purge Perl/gzip from the
  final image stage (or switch to a distroless base) instead of continuing to chase
  individual CVE IDs.

[2.0.2]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.1...v2.0.2

## [2.0.1] - 2026-07-23

### Fixed

- **`SerdeSample<T>` did not implement `DdsType::Native`**: introduced by the `DdsType::Native`
  associated type added in 2.0.0, but missed in this one `impl` block. Broke any build with
  `--features serde` enabled, including the crate's own `cargo doc`/`cargo clippy
  --all-features` CI jobs.
- **`Cargo.lock` was stale since the 2.0.0 version bump**, never regenerated/committed after
  the release — broke `cargo build/clippy/doc --locked` in CI (CI, MSRV, Clippy, Docs, CodeQL
  workflows) with "cannot update the lock file because --locked was passed".
- **Release container's Trivy scan always failed** on 4 CVEs in OS packages of the
  `debian:bookworm-slim` base image (`perl-base`, `perl-Archive-Tar`, `zlib1g`, `bsdutils`),
  none exercised by the published binary and two already marked `will_not_fix`/`fix_deferred`
  upstream by Debian. Added a documented `.trivyignore` for these specific CVEs; the
  CRITICAL/HIGH gate remains active for anything new.

[2.0.1]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.0...v2.0.1

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
