# Stage 1: Build cyclonedds-rust CLI with all dependencies
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y \
    cmake \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/cyclonedds-rust

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY cyclonedds-src/Cargo.toml cyclonedds-src/
COPY cyclonedds-rust-sys/Cargo.toml cyclonedds-rust-sys/
COPY cyclonedds-derive/Cargo.toml cyclonedds-derive/
COPY cyclonedds-build/Cargo.toml cyclonedds-build/
COPY cyclonedds-idlc/Cargo.toml cyclonedds-idlc/
COPY cyclonedds-cli/Cargo.toml cyclonedds-cli/
COPY cyclonedds-bench/Cargo.toml cyclonedds-bench/
COPY cyclonedds-test-suite/Cargo.toml cyclonedds-test-suite/
COPY cyclonedds/Cargo.toml cyclonedds/
COPY cargo-cyclonedds/Cargo.toml cargo-cyclonedds/

# Create dummy lib.rs files so cargo can resolve the workspace
RUN mkdir -p cyclonedds-src/src && echo "" > cyclonedds-src/src/lib.rs \
    && mkdir -p cyclonedds-rust-sys/src && echo "" > cyclonedds-rust-sys/src/lib.rs \
    && mkdir -p cyclonedds-derive/src && echo "" > cyclonedds-derive/src/lib.rs \
    && mkdir -p cyclonedds-build/src && echo "" > cyclonedds-build/src/lib.rs \
    && mkdir -p cyclonedds-idlc/src && echo "" > cyclonedds-idlc/src/lib.rs \
    && mkdir -p cyclonedds-bench/src && echo "" > cyclonedds-bench/src/lib.rs \
    && mkdir -p cyclonedds-test-suite/src && echo "" > cyclonedds-test-suite/src/lib.rs \
    && mkdir -p cyclonedds/src && echo "" > cyclonedds/src/lib.rs \
    && mkdir -p cargo-cyclonedds/src && echo "" > cargo-cyclonedds/src/lib.rs \
    && mkdir -p cyclonedds-cli/src && echo "fn main() {}" > cyclonedds-cli/src/main.rs

RUN cargo build --release --bin cyclonedds-cli -p cyclonedds-cli || true

# Copy full source and build for real
COPY . .
RUN cargo build --release --bin cyclonedds-cli -p cyclonedds-cli

# Stage 2: Minimal runtime image
#
# Distroless, not debian-slim. `.trivyignore` records why: v2.0.1 and v2.0.2 both
# tried to chase CVEs one at a time in the base image's Perl and gzip packages,
# and four different ones appeared and disappeared in that same group of packages
# over two days. None of them is reachable — the published artifact is a single
# Rust binary that never invokes Perl or gzip — so the durable fix the file
# proposes is to stop shipping them at all. This is that fix.
#
# `cc-debian12` carries glibc, libgcc and libstdc++ (a Rust gnu-target binary
# needs libgcc_s) plus ca-certificates, and nothing else: no shell, no package
# manager, no Perl, no gzip, no bsdutils. `libssl3` is gone with them —
# cyclonedds-cli has no OpenSSL dependency unless the `security` feature is
# enabled, which the released binary does not enable.
#
# Two consequences of having no shell:
#   * HEALTHCHECK must be exec form; there is no `sh -c` to run `|| exit 1`.
#     A non-zero exit from the command is already an unhealthy result.
#   * tini is gone. The CLI installs its own SIGINT/SIGTERM handler (`ctrlc`)
#     and is a single process with no children to reap; use `docker run --init`
#     if a reaper is wanted anyway.
#
# NOT BUILT LOCALLY: written without Docker available. The release workflow
# builds and Trivy-scans this image before publishing, so a mistake here fails
# the release rather than shipping — but verify with a real build before tagging.
FROM gcr.io/distroless/cc-debian12:nonroot

COPY --from=builder /usr/src/cyclonedds-rust/target/release/cyclonedds-cli /usr/local/bin/cyclonedds-cli

# `nonroot` is uid/gid 65532, provided by the base image.
USER nonroot
WORKDIR /home/nonroot

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 CMD ["cyclonedds-cli", "health", "__healthcheck"]

ENTRYPOINT ["cyclonedds-cli"]
CMD ["--help"]
