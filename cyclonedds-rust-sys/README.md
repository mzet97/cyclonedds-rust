# cyclonedds-rust-sys

[![crates.io](https://img.shields.io/crates/v/cyclonedds-rust-sys.svg)](https://crates.io/crates/cyclonedds-rust-sys)

Low-level, `unsafe` FFI bindings to the [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds) C library (`ddsc`), generated with `bindgen`.

**Most users want the [`cyclonedds`](https://crates.io/crates/cyclonedds) crate instead.** This crate exposes the raw C API 1:1 (raw `dds_entity_t` handles, raw pointers, no RAII, no lifetime tracking) — it is the foundation `cyclonedds` is built on, not an ergonomic API in its own right.

## What this crate does

- Ships pre-generated bindings (`src/prebuilt_bindings.rs`) so consumers do not need `bindgen`/`libclang` at build time.
- `build.rs` resolves and builds (via CMake) a native `ddsc` library — from `CYCLONEDDS_SRC`, the bundled [`cyclonedds-src`](https://crates.io/crates/cyclonedds-src) crate, a local `vendor/cyclonedds/` checkout, or a system install, in that order — and links against it.
- Runs a per-target **ABI probe**: a small C program is compiled and executed against the same headers used for linking to measure the real `sizeof`/`offsetof` of key DDS types, and a `const` assertion block in `src/lib.rs` **fails the build** if the prebuilt bindings disagree with what was measured on the current target. Cross-compilation (`HOST != TARGET`) requires a pre-generated snapshot at `abi/<target-triple>.rs`, since the probe cannot execute.

See the [workspace README § Build](https://github.com/mzet97/cyclonedds-rust#build) for the full build-resolution order, CMake flags, and cross-compilation caveats — this is the crate whose `build.rs` implements all of that.

## Install

```toml
[build-dependencies]
cyclonedds-src = "1.0"

[dependencies]
cyclonedds-rust-sys = "1.1"
```

## Features

- `security` — forwarded to the CMake build as `-DENABLE_SECURITY=ON -DENABLE_SSL=ON` (requires OpenSSL).
- `internal-ops` — reserved for workspace-internal use.

## Requirements

CMake 3.16+ and a C/C++ toolchain (see the [workspace README](https://github.com/mzet97/cyclonedds-rust#requirements)). Clang/`bindgen` is only needed by maintainers regenerating `prebuilt_bindings.rs`.

## Documentation

- [docs.rs/cyclonedds-rust-sys](https://docs.rs/cyclonedds-rust-sys)
- [Repository](https://github.com/mzet97/cyclonedds-rust)

## License

MIT — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT).
