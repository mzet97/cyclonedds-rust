# cyclonedds-build

[![crates.io](https://img.shields.io/crates/v/cyclonedds-build.svg)](https://crates.io/crates/cyclonedds-build)

`build.rs` helper that compiles OMG IDL files into Rust source using [`cyclonedds-derive`](https://crates.io/crates/cyclonedds-derive) proc-macros (`DdsTypeDerive`, `DdsEnumDerive`, `DdsUnionDerive`, `DdsBitmaskDerive`).

It first tries to shell out to the CycloneDDS C `idlc` compiler (found via `CYCLONEDDS_HOME/bin/idlc` or `PATH`) to obtain a full type descriptor; if `idlc` is not available, it falls back to a built-in, simplified IDL parser ([`src/idl_parser.rs`](src/idl_parser.rs)) covering common IDL constructs.

This crate is the engine behind both [`cyclonedds-idlc`](https://crates.io/crates/cyclonedds-idlc) (the standalone CLI) and [`cargo-cyclonedds`](https://crates.io/crates/cargo-cyclonedds) (the Cargo plugin).

## Usage

```toml
[build-dependencies]
cyclonedds-build = "2.0"
```

```rust
// build.rs
fn main() {
    cyclonedds_build::compile_idl("src/types.idl").unwrap();
}
```

```rust
// src/lib.rs or a module
include!(concat!(env!("OUT_DIR"), "/types.rs"));
```

For more control over the output directory, module name, or `CYCLONEDDS_HOME`, use `compile_idl_with_options` with a `CompileOptions` value instead of `compile_idl`.

## Documentation

- [docs.rs/cyclonedds-build](https://docs.rs/cyclonedds-build)
- [Repository](https://github.com/mzet97/cyclonedds-rust)

## License

MIT — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT).
