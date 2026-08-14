# cyclonedds-derive

[![crates.io](https://img.shields.io/crates/v/cyclonedds-derive.svg)](https://crates.io/crates/cyclonedds-derive)

Procedural derive macros that implement the [`cyclonedds`](https://crates.io/crates/cyclonedds) crate's `DdsType`/`DdsEnumType`/`DdsUnionType` traits (CDR wire-format descriptors, keys, XTypes metadata) from plain Rust structs and enums.

Most users do not depend on this crate directly — `cyclonedds` re-exports its macros under different names to avoid colliding with its own `DdsType` trait:

| This crate exports | Re-exported by `cyclonedds` as |
|---|---|
| `#[derive(DdsType)]` | `#[derive(DdsTypeDerive)]` |
| `#[derive(DdsEnum)]` | `#[derive(DdsEnumDerive)]` |
| `#[derive(DdsUnion)]` | `#[derive(DdsUnionDerive)]` |
| `#[derive(DdsBitmask)]` | `#[derive(DdsBitmaskDerive)]` |

## Example (via `cyclonedds`)

```rust
use cyclonedds::DdsTypeDerive;

#[derive(DdsTypeDerive, Clone, Debug)]
struct SensorReading {
    #[key]
    sensor_id: i32,
    value: f64,
    label: String,
}
```

Attributes supported by `#[derive(DdsType)]`: `#[key]` (mark instance-key fields), `#[dds_enum]` (mark a field as a `DdsEnum`-derived type), `#[dds_typename("module::Struct")]` (override the XTypes type name), `#[dds_type_metadata(info = "CONST", map = "CONST")]` (attach precomputed XCDR2 TypeInformation/TypeMapping blobs).

`#[derive(DdsUnion)]` additionally supports `#[dds_discriminant(Type)]`, `#[dds_case(N)]`, and `#[dds_default]`. `#[derive(DdsBitmask)]` supports `#[bit_bound(N)]` (8/16/32/64).

Full reference: [Type System](https://github.com/mzet97/cyclonedds-rust/blob/main/docs/type-system.md).

## Install

```toml
[dependencies]
cyclonedds-derive = "2.0"
```

## Documentation

- [docs.rs/cyclonedds-derive](https://docs.rs/cyclonedds-derive)
- [Repository](https://github.com/mzet97/cyclonedds-rust)

## License

MIT — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT).
