# cyclonedds-idlc

[![crates.io](https://img.shields.io/crates/v/cyclonedds-idlc.svg)](https://crates.io/crates/cyclonedds-idlc)

Standalone command-line tool that compiles OMG IDL files to Rust source code for CycloneDDS, outside of a `build.rs`. It is a thin CLI wrapper around [`cyclonedds-build`](https://crates.io/crates/cyclonedds-build) (`compile_idl_with_options`) — the same engine used by [`cargo-cyclonedds`](https://crates.io/crates/cargo-cyclonedds).

## Install

```bash
cargo install cyclonedds-idlc
```

## Usage

```bash
cyclonedds-idlc --input types.idl --output-dir src/dds_types/

# Point at a specific CycloneDDS installation containing bin/idlc
cyclonedds-idlc --input types.idl --output-dir src/dds_types/ --cyclonedds-home /path/to/cyclonedds

# Skip the native idlc binary and always use the built-in parser
cyclonedds-idlc --input types.idl --no-idlc
```

Flags (see `cyclonedds-idlc --help` for the authoritative list): `--input <FILE>` (required), `--output-dir <DIR>` (defaults to the current directory), `--cyclonedds-home <DIR>`, `--module-name <NAME>` (defaults to the input file stem), `--no-idlc`.

## Documentation

- [docs.rs/cyclonedds-idlc](https://docs.rs/cyclonedds-idlc)
- [Repository](https://github.com/mzet97/cyclonedds-rust)

## License

MIT — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT).
