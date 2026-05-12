# Contributing to cyclonedds-rust

Thank you for your interest in contributing. This document covers the development workflow, code standards, and pull request process.

## Development setup

### Prerequisites

- **Rust** 1.85+ (MSRV enforced in CI)
- **CMake** 3.10+
- **C/C++ compiler** (GCC, Clang, or MSVC)
- **Git** with submodules support

### Clone and build

```bash
git clone --recurse-submodules https://github.com/mzet97/cyclonedds-rust.git
cd cyclonedds-rust
cargo build --workspace
```

### Run tests

```bash
cargo test --workspace --features async -- --test-threads=1
```

Tests run single-threaded to avoid SIGSEGV from CycloneDDS global domain state in parallel execution.

### Run lints

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings -A missing_docs
```

### Run benchmarks

```bash
cargo bench --workspace
```

## Branching model

- `main` is protected. All changes go through pull requests.
- Branch naming convention:

| Prefix | Purpose |
|--------|---------|
| `feat/` | New features or capabilities |
| `fix/` | Bug fixes |
| `chore/` | Maintenance, CI, dependencies |
| `docs/` | Documentation only |
| `refactor/` | Code restructuring without behavior change |
| `perf/` | Performance improvements |
| `test/` | Test additions or fixes |

Squash-merge is preferred to keep `main` history linear.

## Commit convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:** `feat`, `fix`, `chore`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `revert`

**Scopes:** `sys` (FFI bindings), `api` (safe Rust API), `derive` (proc-macros), `cli` (CLI tools), `build` (build helpers), `idlc` (IDL compiler), `bench` (benchmarks), `wasm` (WASM), `ci` (workflows), `deps` (dependencies)

**Examples:**

```
feat(api): add read_loan zero-copy subscriber support
fix(sys): resolve SIGSEGV in parallel test execution
chore(ci): bump dtolnay/rust-toolchain to stable
docs(api-guide): document request-reply pattern
```

## Code style

- Run `cargo fmt` before every commit. CI enforces formatting.
- Resolve all `cargo clippy` warnings before opening a PR.
- Public API items should have `///` or `//!` doc comments.
- Keep PRs focused: one logical change per PR. Separate refactors from behavior changes.
- Follow the existing module structure in `cyclonedds/src/`.

## Architecture Decision Records

For irreversible decisions (new external dependencies, breaking API changes, MSRV bumps), create an ADR under `docs/adr/`:

```markdown
# ADR-XXXX: <title>

## Status
Proposed | Accepted | Deprecated | Superseded by ADR-YYYY

## Context
What is the technical or product circumstance that motivates this decision?

## Decision
What is the change being made?

## Consequences
What are the positive and negative outcomes?
```

## Pull request checklist

Before requesting review, verify:

- [ ] All tests pass (`cargo test --workspace --features async`)
- [ ] No compiler warnings (`cargo clippy -- -D warnings -A missing_docs`)
- [ ] Code is formatted (`cargo fmt --all -- --check`)
- [ ] `CHANGELOG.md` updated under `[Unreleased]` with the appropriate subsection
- [ ] ADR created if this introduces an irreversible architectural decision
- [ ] No secrets, tokens, or credentials in the diff
- [ ] `Cargo.lock` committed if any dependency changed

## Reporting issues

- **Bugs:** Use the [bug report template](https://github.com/mzet97/cyclonedds-rust/issues/new?template=bug_report.md).
- **Features:** Use the [feature request template](https://github.com/mzet97/cyclonedds-rust/issues/new?template=feature_request.md).
- **Security vulnerabilities:** See [SECURITY.md](SECURITY.md). Do not open public issues for security reports.

## Workspace structure

```
cyclonedds-rust/
  cyclonedds/              # Safe Rust API (main crate)
  cyclonedds-rust-sys/     # FFI bindings (generated via bindgen)
  cyclonedds-derive/       # Proc-macros (DdsType, DdsEnum, etc.)
  cyclonedds-build/        # Build-time IDL helpers
  cyclonedds-idlc/         # IDL compiler backend
  cyclonedds-cli/          # CLI tools
  cargo-cyclonedds/        # Cargo plugin
  cyclonedds-bench/        # Benchmarks
  cyclonedds-test-suite/   # Integration tests
  cyclonedds-wasm/         # WASM bindings (experimental)
  cyclonedds-src/          # Bundled CycloneDDS C source
```
