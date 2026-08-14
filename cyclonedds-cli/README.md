# cyclonedds-cli

[![crates.io](https://img.shields.io/crates/v/cyclonedds-cli.svg)](https://crates.io/crates/cyclonedds-cli)

Command-line diagnostics and interop tools for CycloneDDS domains, built on the [`cyclonedds`](https://crates.io/crates/cyclonedds) crate.

## Install

```bash
cargo install cyclonedds-cli
```

## Subcommands

`ls`, `ps`, `subscribe`, `bridge`, `perf`, `typeof`, `publish`, `discover`, `echo`, `record`, `replay`, `monitor`, `health`, `topology`, `diagnose`, `metrics` — defined in [`src/main.rs`](src/main.rs). Run `cyclonedds-cli --help` or `cyclonedds-cli <subcommand> --help` for the authoritative, up-to-date flags.

```bash
# List discovered participants, publications, and subscriptions
cyclonedds-cli ls --domain-id 0

# Subscribe and print samples as JSON, filtered
cyclonedds-cli subscribe HelloWorld --json --filter "id > 10"

# Publish at a fixed rate
cyclonedds-cli publish HelloWorld --message "hi" --rate 10

# Prometheus-compatible metrics for a topic
cyclonedds-cli metrics HelloWorld

# Full domain state as JSON
cyclonedds-cli diagnose --domain-id 0
```

More examples: [workspace README § CLI Examples](https://github.com/mzet97/cyclonedds-rust#cli-examples).

## Documentation

- [docs.rs/cyclonedds-cli](https://docs.rs/cyclonedds-cli)
- [Repository](https://github.com/mzet97/cyclonedds-rust)

## License

MIT — see [LICENSE-MIT](https://github.com/mzet97/cyclonedds-rust/blob/main/LICENSE-MIT).
