# Type System

The cyclonedds-rust type system maps Rust types to DDS CDR serialization descriptors. This document covers the `DdsType` trait, derive macros, supported types, and CDR encoding details.

## DdsType Trait

Every topic type must implement `DdsType`. Required methods: `type_name()`, `ops()`, `clone_out()`. Optional: `descriptor_size()`, `descriptor_align()`, `key_count()`, `keys()`, `post_key_ops()`. Implement manually (see examples) or use derive macros.

## DdsType Derive

The `#[derive(DdsType)]` macro generates all trait methods automatically from a struct with named fields.

```rust
use cyclonedds::DdsType;

#[derive(DdsType)]
struct SensorReading {
    #[key]
    sensor_id: i32,
    value: f64,
    label: String,
}
```

The macro generates: `type_name()`, `ops()`, `clone_out()`, `descriptor_size()`, `descriptor_align()`, `key_count()`, `keys()`, and `post_key_ops()`.

### Key Fields

Mark fields with `#[key]` to define DDS instance keys:

```rust
#[derive(DdsType)]
struct Measurement {
    #[key]
    device_id: i32,
    #[key]
    timestamp: i64,
    value: f64,
}
```

### Enum Fields

Mark fields with `#[dds_enum]` to indicate the field type is a DDS enum:

```rust
#[derive(DdsEnum)]
enum Status { Ok = 0, Warning = 1, Error = 2 }

#[derive(DdsType)]
struct Reading {
    #[key]
    id: i32,
    #[dds_enum]
    status: Status,
}
```

## DdsEnum Derive

For fieldless (C-like) enums used as DDS enumerations:

```rust
#[derive(DdsEnum)]
enum Color {
    Red = 0,
    Green = 1,
    Blue = 2,
}
```

Generates the `DdsEnumType` trait impl with `max_discriminant()`. Supports explicit and auto-incrementing discriminants. Only non-negative discriminants are supported.

## DdsUnion Derive

Discriminated unions mapping to IDL `union` types. Use attributes to define the discriminant and each case.

```rust
#[derive(DdsUnion)]
#[dds_discriminant(u8)]
enum Value {
    #[dds_case(0)]
    IntValue(i32),
    #[dds_case(1)]
    FloatValue(f64),
    #[dds_case(2)]
    TextValue(String),
    #[dds_default]
    NoneValue(()),
}
```

Attributes:
- `#[dds_discriminant(Type)]` -- required on the enum, sets the discriminant type (bool, u8..u64, i8..i64)
- `#[dds_case(N)]` -- required on each non-default variant, sets the case value
- `#[dds_default]` -- optional, marks the default case (at most one)

Case members support primitives (i8..i64, u8..u64, f32, f64, bool) and String.

## DdsBitmask Derive

Bitmask types mapping to IDL `bitmask` with a configurable bit bound.

```rust
#[derive(DdsBitmask)]
#[bit_bound(8)]
struct Permissions {
    read: bool,
    write: bool,
    execute: bool,
}
```

All fields must be `bool`. The `#[bit_bound(N)]` attribute accepts 8, 16, 32, or 64 (default: 32). Generated methods:

```rust
let p = Permissions { read: true, write: false, execute: true };
let bits = p.to_bits();           // u64
let p2 = Permissions::from_bits(5);
assert!(p.read());               // getter
p.set_write(true);               // setter
```

## Supported Types

| Rust Type | CDR Mapping | Notes |
|-----------|------------|-------|
| `i8` | TYPE_1BY + OP_FLAG_SGN | |
| `u8` | TYPE_1BY | |
| `i16` | TYPE_2BY + OP_FLAG_SGN | |
| `u16` | TYPE_2BY | |
| `i32` | TYPE_4BY + OP_FLAG_SGN | |
| `u32` | TYPE_4BY | |
| `i64` | TYPE_8BY + OP_FLAG_SGN | |
| `u64` | TYPE_8BY | |
| `f32` | TYPE_4BY + OP_FLAG_FP | |
| `f64` | TYPE_8BY + OP_FLAG_FP | |
| `bool` | TYPE_1BY | |
| `String` | TYPE_STR | DDS string type |
| `[T; N]` | TYPE_ARR | Primitive/string/composite arrays |
| `Vec<T>` | TYPE_SEQ | Primitives, enums, strings, composites |
| `DdsSequence<T>` | TYPE_SEQ | DDS-native sequence |
| `DdsBoundedSequence<T, N>` | TYPE_BSQ | Bounded sequence |
| `Option<T>` | OP_FLAG_OPT + OP_FLAG_EXT | Optional member (not keyable) |
| Nested struct | TYPE_EXT | Any struct implementing `DdsType` |
| DdsEnum type | TYPE_ENU | Fieldless enum with `DdsEnumType` |

## Nested Types

```rust
#[derive(DdsType)]
struct Position { x: f64, y: f64, z: f64 }

#[derive(DdsType)]
struct Vehicle {
    #[key]
    id: i32,
    pos: Position,       // nested struct (TYPE_EXT)
}
```

Keys propagate from nested structs to the parent.

## CDR Encoding

| Version | Constant | Use Case |
|---------|----------|----------|
| XCDR1 | `CdrEncoding::Xcdr1` | Plain (Final) IDL types |
| XCDR2 | `CdrEncoding::Xcdr2` | Appendable/Mutable types |

```rust
let bytes = CdrSerializer::serialize(&sample, CdrEncoding::Xcdr1)?;
```

## Extensibility

| Kind | Description |
|------|-------------|
| Final | No extension (default for DdsType derive) |
| Appendable | New members appended at end |
| Mutable | Members in any order (requires XCDR2) |

## Ops Quick Reference

| Constant | Meaning |
|----------|---------|
| `OP_ADR` | Address-of-data instruction |
| `OP_RTS` | Return from sub-sequence |
| `OP_JEQ4` | Jump-equal (union cases) |
| `OP_DLC` | Delete container |
| `OP_MID` | Member ID (appendable/mutable) |
| `OP_FLAG_KEY` / `OPT` / `EXT` / `FP` / `SGN` / `MU` | Flags: key, optional, extended, float, signed, mutable union |
| `TYPE_1BY` .. `TYPE_8BY` | Primitive size classes |
| `TYPE_STR` / `SEQ` / `BSQ` / `ARR` / `ENU` / `UNI` / `EXT` | String, sequence, bounded sequence, array, enum, union, extended |
| `SUBTYPE_STR` / `SEQ` / `BST` / `STU` / `ENU` | Sub-type variants |
