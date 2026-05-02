# DDS Security Production Hardening

This guide covers production-ready practices for DDS Security with `cyclonedds-rust`.

## Certificate Lifecycle

### Initial Setup

1. Generate a root CA (keep offline):
   ```bash
   openssl req -x509 -newkey rsa:4096 -keyout ca-key.pem -out ca-cert.pem -days 3650 -nodes
   ```

2. Generate participant identity certificates:
   ```bash
   openssl req -newkey rsa:2048 -keyout participant-key.pem -out participant-csr.pem -nodes
   openssl x509 -req -in participant-csr.pem -CA ca-cert.pem -CAkey ca-key.pem -out participant-cert.pem -days 365 -CAcreateserial
   ```

3. Create governance and permissions XML files (see OMG DDS Security spec).

### Certificate Rotation

**Without downtime:**

1. Generate new certificates with overlapping validity periods.
2. Update certificate files on disk.
3. Call `SecurityConfig::reload()` and `validate()`.
4. Recreate `DomainParticipant` with the new config.
5. Old participants with valid (but expiring) certificates continue to communicate during the overlap period.

```rust
use cyclonedds::{DomainParticipant, QosBuilder, SecurityConfig};

fn rotate_certificates() {
    let new_security = SecurityConfig::new()
        .identity_ca("certs/new_identity_ca.pem")
        .identity_certificate("certs/new_cert.pem")
        .identity_private_key("certs/new_key.pem")
        .governance("governance.xml")
        .permissions("permissions.xml")
        .permissions_ca("certs/new_permissions_ca.pem")
        .crl("certs/revoked.pem")
        .validate()
        .expect("new certificates must be valid");

    let qos = QosBuilder::new()
        .security(new_security)
        .build()
        .unwrap();

    // Gracefully recreate participant
    let participant = DomainParticipant::with_qos(0, Some(&qos)).unwrap();
}
```

### Certificate Revocation Lists (CRL)

Enable CRL checking by setting the `crl` path:

```rust
let security = SecurityConfig::new()
    .identity_ca("certs/ca.pem")
    .identity_certificate("certs/cert.pem")
    .identity_private_key("certs/key.pem")
    .crl("certs/crl.pem")  // PEM-formatted CRL
    .validate()
    .unwrap();
```

CycloneDDS checks the CRL during handshake. Revoked certificates are rejected.

To update the CRL without restarting:
1. Overwrite `crl.pem` with the updated list.
2. Recreate the participant (DDS Security does not support hot-reload of CRLs).

## Validation Checklist

Before deploying to production, run `SecurityConfig::validate()` which checks:
- [ ] All configured files exist and are readable.
- [ ] All PEM files contain valid `-----BEGIN` markers.
- [ ] CRL file (if configured) exists and is readable.

## Penetration Testing

### Unauthorized Participant Rejection

A participant without valid credentials must be rejected. Test with:

```rust
#[test]
fn unauthorized_participant_is_rejected() {
    // Authorized participant with valid certs
    let authorized_qos = QosBuilder::new()
        .security(SecurityConfig::new()
            .identity_ca("certs/ca.pem")
            .identity_certificate("certs/valid.pem")
            .identity_private_key("certs/valid.key")
            .governance("governance.xml")
            .permissions("permissions.xml")
            .permissions_ca("certs/ca.pem"))
        .build()
        .unwrap();

    let authorized = DomainParticipant::with_qos(0, Some(&authorized_qos)).unwrap();

    // Unauthorized participant without certs
    let unauthorized = DomainParticipant::new(0).unwrap();

    // In a secured domain, the unauthorized participant should not be able
    // to discover or communicate with the authorized participant.
    // Verify by checking matched endpoints after a delay.
}
```

## OpenSSL Requirements

- **Linux**: `libssl-dev` or `openssl-devel`
- **macOS**: `brew install openssl`
- **Windows**: Install OpenSSL and set `OPENSSL_ROOT_DIR` environment variable.

## CI/CD Security Tests

Enable security tests in a dedicated CI job with OpenSSL installed:

```yaml
- name: Security tests
  run: cargo test --features security --locked
  env:
    OPENSSL_ROOT_DIR: /usr/lib/ssl
```

## References

- [OMG DDS Security Specification v1.1](https://www.omg.org/spec/DDS-SECURITY/)
- [CycloneDDS Security Documentation](https://cyclonedds.io/docs/cyclonedds/latest/security.html)
