# DDS Security Guide

This guide explains how to use DDS Security with `cyclonedds-rust`.

## Overview

DDS Security (DDS-Security spec v1.1) provides:
- **Authentication** — verify participant identity via certificates
- **Access Control** — restrict who can publish/subscribe to which topics
- **Cryptography** — encrypt and sign data on the wire

CycloneDDS implements these via plugins configured through QoS properties.

## Prerequisites

1. OpenSSL installed on your system
2. Build with the `security` feature:
   ```bash
   cargo build --features security
   ```

## Quick Start

### 1. Generate Certificates

A helper script is provided:

```bash
cd examples/security
../../scripts/generate-certs.sh
```

This creates:
- `identity_ca_cert.pem` — Identity CA certificate
- `permissions_ca_cert.pem` — Permissions CA certificate
- `participant_cert.pem` — Participant identity certificate
- `participant_key.pem` — Participant private key

### 2. Prepare Governance and Permissions

Two XML documents are required:

**`governance.xml`** — Defines security policies for domains:
- Encryption level for discovery, liveliness, and RTPS traffic
- Whether access control is enforced
- Default protection kind (SIGN vs ENCRYPT)

**`permissions.xml`** — Grants or denies access to specific participants:
- Subject name (matches certificate)
- Validity period
- Allowed domains, topics, publish/subscribe rights

Example files are in `examples/security/`.

### 3. Configure Security in Code

```rust
use cyclonedds::{DomainParticipant, QosBuilder, SecurityConfig};

let security = SecurityConfig::new()
    .identity_ca("certs/identity_ca_cert.pem")
    .identity_certificate("certs/participant_cert.pem")
    .identity_private_key("certs/participant_key.pem")
    .governance("certs/governance.xml")
    .permissions("certs/permissions.xml")
    .permissions_ca("certs/permissions_ca_cert.pem");

let qos = QosBuilder::new()
    .security(security)
    .build()?;

let participant = DomainParticipant::with_qos(0, Some(&qos))?;
```

### 4. Run the Example

Terminal 1 (subscriber):
```bash
cargo run --example security_sub --features security
```

Terminal 2 (publisher):
```bash
cargo run --example security_pub --features security
```

If certificates are missing or invalid, participant creation will fail with `DdsError::BadParameter` or `DdsError::NotAllowedBySecurity`.

## SecurityConfig API

### Builder Methods

| Method | Description |
|--------|-------------|
| `identity_ca(path)` | Path to Identity CA certificate |
| `identity_certificate(path)` | Path to participant certificate |
| `identity_private_key(path)` | Path to participant private key |
| `governance(path)` | Path to Governance XML |
| `permissions(path)` | Path to Permissions XML |
| `permissions_ca(path)` | Path to Permissions CA certificate |
| `auth_plugin(name)` | Override auth plugin (default: `dds.sec.auth.builtin.PKI-DH`) |
| `access_plugin(name)` | Override access plugin (default: `dds.sec.access.builtin.Access-Permissions`) |
| `crypto_plugin(name)` | Override crypto plugin (default: `dds.sec.crypto.builtin.AES-GCM-GMAC`) |

### Applying to QoS

```rust
let qos = QosBuilder::new()
    .security(security_config)
    .build()?;
```

## Troubleshooting

### "Could NOT find OpenSSL"

CMake cannot find OpenSSL. Install it:
- **Ubuntu/Debian**: `sudo apt-get install libssl-dev`
- **macOS**: `brew install openssl`
- **Windows**: Install OpenSSL and set `OPENSSL_ROOT_DIR`

### "Not allowed by security"

Participant creation fails because:
- Certificate expired
- Subject name in permissions does not match certificate
- Governance/permissions XML malformed
- Required file path incorrect

### "Some security properties specified but Cyclone build does not include security"

You are building without the `security` feature:
```bash
cargo build --features security
```

## See Also

- [DDS Security Spec v1.1](https://www.omg.org/spec/DDS-SECURITY/)
- CycloneDDS security documentation
- `examples/security_pub.rs` and `examples/security_sub.rs`
