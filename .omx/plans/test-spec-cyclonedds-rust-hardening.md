# Test Spec: cyclonedds-rust hardening and QoS provider support

## Verification Matrix
- Clean `cyclonedds-sys` rebuild after build script changes.
- Workspace tests with real CycloneDDS runtime.
- All-features test pass.
- Example compilation.
- Clippy with warnings denied.
- Release build.
- Public FFI coverage audit.

## Runtime Assertions
- `QosProvider::new` loads inline DDS XML.
- Participant QoS loaded from provider can be applied with `DomainParticipant::with_qos`.
- Topic QoS loaded from provider can be applied with `create_topic_with_qos`.
- Writer/reader QoS loaded from provider can be applied and exchange samples successfully.
- Existing regression suite remains green.
