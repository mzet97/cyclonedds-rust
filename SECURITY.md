# Security Policy

## Supported versions

Only the latest release is supported with security fixes.

| Version | Supported |
|---------|-----------|
| 1.8.x   | Yes       |
| < 1.8.0 | No        |

## Reporting a vulnerability

**Do not open a public GitHub issue.**

Report security vulnerabilities to:

**Matheus.zeitune.developer@gmail.com**

Include the following in your report:

- Description of the vulnerability and its impact
- Steps to reproduce or a proof-of-concept
- Affected version (commit hash or release tag)
- Relevant logs or stack traces
- Any suggested mitigation

## Response targets

| Step | Target |
|------|--------|
| Acknowledgment | 2 business days |
| Initial triage + severity rating (CVSS v3.1) | 5 business days |
| Fix in `main` (high/critical) | 30 calendar days |
| Fix in `main` (medium/low) | 90 calendar days |
| Public disclosure | After fix release is published |

We use [CVSS v3.1](https://www.first.org/cvss/v3.1/specification-document) for severity scoring. Disclosure is coordinated with the reporter.

## Threat model (in scope)

- Memory safety violations in the Rust API surface (`cyclonedds` crate)
- Undefined behavior crossing the FFI boundary (`cyclonedds-rust-sys`)
- DDS Security misconfiguration (certificate validation, governance/permissions XML)
- CDR deserialization panics or out-of-memory conditions from malformed data
- Supply chain attacks via dependency confusion or compromised crates
- Privilege escalation through DDS discovery or participant impersonation
- Timing side-channels in security-critical operations

## Out of scope

- Denial of service attacks against the CycloneDDS C daemon (upstream: [Eclipse CycloneDDS](https://github.com/eclipse-cyclonedds/cyclonedds))
- Vulnerabilities in dependencies already fixed in the latest version
- Physical access attacks
- Social engineering
- Issues in experimental features (`no_std`, WASM) unless they affect the main `std` path

## Security features

This project supports DDS Security via the `security` feature flag:

```toml
[dependencies]
cyclonedds = { version = "1.8", features = ["security"] }
```

See [docs/security-guide.md](docs/security-guide.md) and [docs/security-production.md](docs/security-production.md) for configuration details.
