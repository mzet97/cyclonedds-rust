# cyclonedds-src

[![crates.io](https://img.shields.io/crates/v/cyclonedds-src.svg)](https://crates.io/crates/cyclonedds-src)

Vendored copy of the [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds) C source tree, packaged as a Rust crate so it can be pulled in as a `build-dependency` and built from source with CMake — without requiring a system-wide CycloneDDS install.

**This crate has no Rust API to speak of and is not meant to be used directly.** It exists purely so [`cyclonedds-rust-sys`](https://crates.io/crates/cyclonedds-rust-sys)'s `build.rs` can locate and compile the C library. If you are building a Rust application, depend on [`cyclonedds`](https://crates.io/crates/cyclonedds) instead.

## What it provides

Two functions over the bundled source tree at `src/cyclonedds/`:

```rust
/// Directory containing the CycloneDDS C source tree.
pub fn source_dir() -> std::path::PathBuf;

/// Directory containing the public C headers (`src/core/ddsc/include`).
pub fn include_dir() -> std::path::PathBuf;
```

## Usage

```toml
[build-dependencies]
cyclonedds-src = "1.0"
```

```rust
// build.rs
let src = cyclonedds_src::source_dir();
// configure and build with cmake, e.g. via std::process::Command
```

## Documentation

- [docs.rs/cyclonedds-src](https://docs.rs/cyclonedds-src)
- [Repository](https://github.com/mzet97/cyclonedds-rust)

## License

This crate's own Rust code is MIT-licensed — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT). The bundled CycloneDDS C source under `src/cyclonedds/` retains its own upstream license (see `src/cyclonedds/LICENSE`).
