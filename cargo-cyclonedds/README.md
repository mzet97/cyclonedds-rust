# cargo-cyclonedds

[![crates.io](https://img.shields.io/crates/v/cargo-cyclonedds.svg)](https://crates.io/crates/cargo-cyclonedds)

Cargo subcommand that generates Rust types from OMG IDL files for CycloneDDS. It wraps the same engine as [`cyclonedds-idlc`](https://crates.io/crates/cyclonedds-idlc) — [`cyclonedds-build`](https://crates.io/crates/cyclonedds-build)'s `compile_idl_with_options` — but plugs into the `cargo` CLI for one-off generation from a shell.

Use this for ad-hoc or checked-in generated code. For code generated automatically on every build, call `cyclonedds-build` from your `build.rs` instead.

## Install

```bash
cargo install cargo-cyclonedds
```

## Usage

```bash
# Generate into the current directory
cargo cyclonedds generate HelloWorld.idl

# Generate into a specific directory
cargo cyclonedds generate types.idl --output-dir src/dds_types/

# Point at a CycloneDDS installation containing bin/idlc
cargo cyclonedds generate types.idl --cyclonedds-home /path/to/cyclonedds

# Always use the built-in parser, never the native idlc binary
cargo cyclonedds generate types.idl --no-idlc
```

Flags for `generate` (see `cargo cyclonedds generate --help` for the authoritative list): `<IDL_FILE>` (positional, required), `-o/--output-dir <DIR>` (defaults to the current directory), `--cyclonedds-home <DIR>`, `-m/--module-name <NAME>`, `--no-idlc`.

The generated code depends on the [`cyclonedds`](https://crates.io/crates/cyclonedds) crate, which must be a dependency of the consuming project.

## Documentation

- [Repository](https://github.com/mzet97/cyclonedds-rust)
- [Type System guide](https://github.com/mzet97/cyclonedds-rust/blob/main/docs/type-system.md)

## License

MIT — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT).
