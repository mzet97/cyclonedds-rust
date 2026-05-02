# Fuzzing

This project uses [`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) for automated fuzz testing of CDR deserialization.

## Prerequisites

`cargo-fuzz` requires:
- **Linux** or **macOS** (Windows is not officially supported)
- A **nightly** Rust toolchain
- `clang` installed

Install `cargo-fuzz`:

```bash
cargo install cargo-fuzz
```

## Running the fuzzer

```bash
# Switch to nightly
rustup default nightly

# Run the CDR deserialization fuzz target
cd fuzz
cargo fuzz run cdr_deserialize

# Run with a timeout (e.g., 10 minutes)
cargo fuzz run cdr_deserialize -- -max_total_time=600

# Run with a max number of runs
cargo fuzz run cdr_deserialize -- -runs=1000000
```

## Fuzz targets

### `cdr_deserialize`

Feeds arbitrary byte slices to `CdrDeserializer::<FuzzSample>::deserialize` for both XCDR1 and XCDR2 encodings.

The goal is to detect:
- **Panics** caused by malformed CDR input
- **Memory safety issues** in the deserialization path
- **Infinite loops** or hangs

### `FuzzSample` type

```rust
#[repr(C)]
#[derive(DdsTypeDerive)]
struct FuzzSample {
    id: i32,
    payload: [u8; 64],
}
```

This is a representative DDS type with a primitive field and a fixed-size array.

## CI integration

Fuzzing is **not enabled as a CI gate** because it requires significant time and resources. However, you can run it periodically (e.g., nightly) using GitHub Actions or similar:

```yaml
name: Fuzz
on:
  schedule:
    - cron: '0 0 * * *'  # nightly
jobs:
  fuzz:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: rustup default nightly
      - run: cargo install cargo-fuzz
      - run: cd fuzz && cargo fuzz run cdr_deserialize -- -max_total_time=3600
```

## Minimizing crashes

If the fuzzer finds a crash, minimize the reproducer:

```bash
cd fuzz
cargo fuzz tmin cdr_deserialize crash-<hash>
```

## Corpus

The fuzzer automatically builds a corpus of interesting inputs in `fuzz/corpus/cdr_deserialize/`. You can seed it with valid CDR samples:

```bash
mkdir -p fuzz/corpus/cdr_deserialize
# Add valid serialized samples
cp valid_sample.cdr fuzz/corpus/cdr_deserialize/
```

## Adding new fuzz targets

1. Create `fuzz/fuzz_targets/my_target.rs`
2. Add the target to `fuzz/Cargo.toml`
3. Run with `cargo fuzz run my_target`

## Notes

- Fuzzing is computationally expensive. Run it on dedicated machines when possible.
- The `libfuzzer` engine uses coverage-guided fuzzing, so it improves over time.
- For long-running fuzzing campaigns, consider using a fuzzing cluster or cloud VMs.
