# AGENTS.md — cyclonedds-rust knowledge base for coding agents

Read this file before changing code. Audit findings, PR triage and the
evidence behind them live in [Notes.md](Notes.md). The soundness history
lives in [docs/soundness-backlog.md](docs/soundness-backlog.md) — treat it
as an input, not as truth: re-verify every claim against the code at HEAD.

## What this project is

Safe, idiomatic Rust bindings for Eclipse CycloneDDS (OMG DDS pub/sub
middleware for distributed systems). Workspace version `3.0.0`, edition
2021, MSRV Rust 1.85. Repository: `github.com/mzet97/cyclonedds-rust`.

The core risk surface is the FFI boundary: a safe API (`cyclonedds`) over
generated C bindings (`cyclonedds-rust-sys`) over a vendored CycloneDDS C
library built with CMake at build time. Most defects that matter are
memory-safety or wire-format defects crossing that boundary, and most of
them arrive as remote input (a peer with a different IDL revision).

## Workspace map

| Crate | Role | Published |
|---|---|---|
| `cyclonedds` | Safe high-level API: entities, QoS (26+), listeners, WaitSet, async streams, loans, dynamic types, DDS Security config | yes |
| `cyclonedds-rust-sys` | Unsafe FFI bindings. Uses checked-in `src/prebuilt_bindings.rs`; **never runs bindgen at build time** | yes |
| `cyclonedds-src` | Bundled CycloneDDS C source (currently **11.0.1**, `CMakeLists.txt: project(CycloneDDS VERSION 11.0.1)`) | yes |
| `cyclonedds-derive` | Proc-macros: `DdsTypeDerive`, `DdsEnumDerive`, `DdsUnionDerive`, `DdsBitmaskDerive` (re-exported by `cyclonedds`) | yes |
| `cyclonedds-build` | `build.rs` helper: IDL → Rust codegen (`compile_idl`) | yes |
| `cyclonedds-idlc` / `cargo-cyclonedds` | CLIs wrapping `cyclonedds-build` | yes |
| `cyclonedds-cli` | Diagnostics CLI (`ls`, `ps`, `subscribe`, `publish`, `monitor`, `health`, `topology`, …) | yes |
| `cyclonedds-wasm`, `dds-wasm-*`, `dds-battleship` | Experimental WASM/WebSocket transport and demo apps | internal |
| `cyclonedds-test-suite` | Integration tests (one file per concern in `tests/`) | no |
| `cyclonedds-bench`, `dds-demo-test` | Criterion benches, manual demos | no |
| `fuzz/` | Separate `cargo-fuzz` workspace, **not** a workspace member | no |

Dependency direction: app → `cyclonedds` → `cyclonedds-rust-sys` → C
library. `cyclonedds` also re-exports the derives from `cyclonedds-derive`.

## Build and test

```bash
cargo build --workspace
cargo test --workspace            # DDS tests share domain state; CI uses --test-threads=1 in places
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings -A missing_docs
cargo fmt --all -- --check
bash scripts/ci/check-unsafe-contracts.sh   # unsafe inventory gate
cargo audit                       # RustSec; cargo-deny config in deny.toml
```

Requirements: Rust 1.85+, CMake 3.16+, a C compiler. Clang is only needed
to regenerate bindings (`scripts/regenerate-bindings.sh`), not to build.

Known local pitfall: a stale CMake cache in `target/` (e.g. after the
checkout moves path) fails `cyclonedds-rust-sys`'s build script with
"CMakeCache.txt directory … is different". Use a fresh
`CARGO_TARGET_DIR` instead of debugging the build script.

## How the FFI layer actually works (invariants)

1. `cyclonedds-rust-sys/build.rs` copies `src/prebuilt_bindings.rs` to
   `$OUT_DIR/bindings.rs`, stripping the embedded libc layout asserts
   (generated for macOS/Linux libc types; they do not compile elsewhere).
2. It then compiles and **runs an ABI probe** against the real headers for
   the current target and emits `abi_probe.rs`. Const assertions in
   `src/lib.rs` fail the build on any size/align/offset mismatch. Silent
   ABI corruption is a compile error by design — do not weaken this.
3. Cross-compiling (`HOST != TARGET`) needs a committed snapshot in
   `cyclonedds-rust-sys/abi/<target-triple>.rs`; only the Windows MSVC
   snapshot is committed. Linux/macOS snapshots come from CI artifacts
   (`scripts/capture-abi-snapshot.sh`).
4. Source resolution order: `CYCLONEDDS_SRC` env → `cyclonedds-src` crate
   → `vendor/cyclonedds` submodule → system `libddsc` (unsupported alone:
   the probe needs matching headers). `vendor/` and `cyclonedds-src` have
   historically carried different CycloneDDS patch versions; the build
   script prints which source/version it linked. Check that line before
   citing any C source.
5. `DdsType::Native` is the wire layout CycloneDDS reads. The Rust type
   and its `Native` differ whenever a field is `String`/`Vec`/nested
   composite. Handing `&T` to C where `&T::Native` is expected is the
   single most damaging historical bug class here (heap corruption via
   misread `DdsString`/`DdsSequence`). Any new reader/writer/loan path
   must route through `write_to_native` / `clone_out`, never through a
   raw cast of the user type.
6. `DdsType::clone_out` is fallible (`DdsResult<Self>`) on purpose: an
   undecodable sample (unknown union discriminator, invalid pointer) is
   remote input and must become a skipped `Err`, never a panic across the
   FFI boundary. Every implementation — default, derive-generated, and
   hand-written — must reject null/misaligned pointers with
   `DdsError::BadParameter` before dereferencing.
7. Entity lifetime: every entity holds an `Arc<OwnedEntity>` for each
   ancestor, so a parent cannot be deleted while a child is alive.
   Constructors take `&DomainParticipant` / `&DataReader<T>` etc.
   Declaration order must never matter; if it does, that is the bug.
8. Loans: `DataWriter::request_loan` zero-initializes exactly
   `size_of::<T::Native>()` bytes (what `dds_request_loan` allocated).
   `WriteLoan` holds a `NonNull`; dropping an unwritten loan runs the
   `Native` destructor and returns the loan. Do not reintroduce raw
   nullable sample pointers here.

## Safety policy (hard gate)

[SAFETY.md](SAFETY.md) is enforced by `scripts/ci/check-unsafe-contracts.sh`:
it counts lines containing `unsafe` per tracked `.rs` file and diffs
against `scripts/ci/unsafe-inventory.txt`. Any drift fails CI. If a change
genuinely needs new `unsafe`:

- keep it behind the smallest safe API boundary;
- add a nearby `// SAFETY:` contract (pointer validity, lifetime,
  aliasing, alignment, initialization, ownership, unwind, thread-safety);
- add a regression test that reaches the public safe API;
- update `scripts/ci/unsafe-inventory.txt` in the same change;
- pure-Rust paths run under Miri (strict provenance + Tree Borrows);
  FFI/loans/callbacks/dynamic-XTypes paths run under AddressSanitizer.

Generated bindgen output (`prebuilt_bindings.rs`, `$OUT_DIR/bindings.rs`)
is inventoried but never edited by hand. Fix lint/ABI issues in
`build.rs`, in the generator, or with a scoped `#[allow(...)]` on the
bindings module in `cyclonedds-rust-sys/src/lib.rs`.

## Dependency and CI rules that bite

- **CodeQL actions bump as a pair.** `github/codeql-action/init` and
  `.../analyze` must land in the same PR (precedent: PR #41). Dependabot
  opens them separately; merging only one fails `Analyze` with a
  configuration error (observed on PRs #57/#59). Close the singles as
  superseded by a paired bump.
- **`bindgen` is build-optional** (`cyclonedds-rust-sys`, feature-gated,
  used only by maintainers regenerating bindings). A Dependabot bump of
  `bindgen` can break against the `syn` major already in the graph
  (PR #48: bindgen 0.73.2 vs syn 3.0.3 → `syn::file::File: ParseQuote`
  not satisfied). Do not merge a bindgen bump that fails `clippy`/`docs`;
  it is not on the default build path and can wait for an upstream fix.
- New Clippy/rustc lints land on generated bindings first. The current
  known case is `suspicious_runtime_symbol_definitions` on the
  bindgen-declared `malloc`/`realloc` (declarations only, no
  `#[no_mangle]`); it is allowed, with `unknown_lints`, in
  `cyclonedds-rust-sys/src/lib.rs`. Add future generated-code lints the
  same way — scoped allow, never by editing generated output.
- Dependabot PRs with all-green checks and a patch/minor bump are
  mergeable as-is. A failing check on a Dependabot PR is often **not**
  caused by the bump (e.g. the bindings Clippy lint blocked the `libc`
  and actions-group bumps). Read the failed log before blaming the diff.
- `deny.toml`: wildcards denied, unknown registries/git denied. The
  container image is distroless; `.trivyignore` is intentionally empty —
  remove packages instead of suppressing CVEs.

## Conventions

- Errors: `DdsError` / `DdsResult<T>`; map every `dds_*` return code via
  `crate::error::check`. Never `panic!`/`expect` on a path reachable
  from wire data or FFI results.
- Tests: one concern per file in `cyclonedds-test-suite/tests/`, named
  after the behavior (`parent_ownership.rs`, `write_loan.rs`,
  `clone_out_invalid_pointer.rs`). DDS integration tests must use
  domain 0 or a helper (`unique_topic`) — arbitrary high domain ids can
  fail on some hosts for environmental port reasons, which masquerades
  as a regression.
- Changelog: Keep-a-Changelog format in `CHANGELOG.md`, entry under
  `[Unreleased]` for every behavior or security fix.
- Docs and maintainer-facing text: technical English. User-facing audit
  reports: PT-BR. Code comments: English, concise, no deliberation.

## PR and vulnerability triage procedure

1. `gh pr list --repo mzet97/cyclonedds-rust --state all --limit 100`
   and separate Dependabot bumps from human/security PRs.
2. Code scanning: `gh api --paginate
   'repos/mzet97/cyclonedds-rust/code-scanning/alerts?per_page=100'`.
   Group by `rule.id` and `state` (`open` / `fixed` / `dismissed`).
   As of the 2026-10 audit: 176 alerts, 0 open, 145 fixed, 31 dismissed
   as `mitigated` — all `rust/access-invalid-pointer` or
   `rust/unused-variable`. Verify dismissed locations in the current
   code before trusting the dismissal (see Notes.md F-01).
3. Dependabot alerts: `gh api
   repos/mzet97/cyclonedds-rust/dependabot/alerts` (0 open at audit
   time) plus a local `cargo audit`.
4. For every failing check on an open PR, read the failed log
   (`gh run view <id> --log-failed`) and decide: caused by the diff,
   pre-existing on `main`, or toolchain drift. Only the first blocks
   the PR on its own merits.
5. Fix root causes on `main`; do not patch Dependabot branches.
