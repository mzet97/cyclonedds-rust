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
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    tini \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 1001 cyclonedds \
    && useradd --uid 1001 --gid cyclonedds --shell /bin/bash --create-home cyclonedds

COPY --from=builder /usr/src/cyclonedds-rust/target/release/cyclonedds-cli /usr/local/bin/cyclonedds-cli

USER cyclonedds
WORKDIR /home/cyclonedds

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
    CMD cyclonedds-cli health __healthcheck || exit 1

ENTRYPOINT ["tini", "--"]
CMD ["cyclonedds-cli", "--help"]
