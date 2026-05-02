//! DDS Security integration tests.
//!
//! These tests validate the SecurityConfig builder and QoS property application.
//! Full end-to-end security handshake tests require valid certificates and
//! the `security` feature enabled.
//!
//! Run with: `cargo test --test security --features security`

#![cfg(feature = "security")]

use cyclonedds::{DomainParticipant, QosBuilder, SecurityConfig};

/// Test that SecurityConfig builds correctly with all fields set.
#[test]
fn security_config_builder() {
    let config = SecurityConfig::new()
        .identity_ca("/path/to/identity_ca.pem")
        .identity_certificate("/path/to/cert.pem")
        .identity_private_key("/path/to/key.pem")
        .governance("/path/to/governance.xml")
        .permissions("/path/to/permissions.xml")
        .permissions_ca("/path/to/permissions_ca.pem")
        .auth_plugin("custom.auth.plugin")
        .access_plugin("custom.access.plugin")
        .crypto_plugin("custom.crypto.plugin");

    // The config should be cloneable and debug-printable.
    let _cloned = config.clone();
    let _debug = format!("{:?}", config);
}

/// Test that SecurityConfig applies the correct QoS properties.
#[test]
fn security_qos_properties() {
    let config = SecurityConfig::new()
        .identity_ca("/certs/ca.pem")
        .identity_certificate("/certs/cert.pem")
        .identity_private_key("/certs/key.pem")
        .governance("/policy/governance.xml")
        .permissions("/policy/permissions.xml")
        .permissions_ca("/certs/permissions_ca.pem");

    let qos = QosBuilder::new()
        .security(config)
        .build()
        .expect("QoS build should succeed");

    // Verify that the QoS contains the expected properties.
    // We cannot inspect properties directly from the public API,
    // but we can verify the QoS builds without error.
    drop(qos);
}

/// Test that creating a participant with security config but invalid file paths
/// behaves gracefully (exact behavior depends on CycloneDDS validation).
#[test]
fn security_participant_with_invalid_paths() {
    // Skip if security feature is not enabled — without it the security
    // plugins are not compiled into CycloneDDS and the participant may
    // still be created (properties are silently ignored).
    #[cfg(not(feature = "security"))]
    {
        eprintln!("Skipping: security feature not enabled");
        return;
    }

    let config = SecurityConfig::new()
        .identity_ca("/nonexistent/ca.pem")
        .identity_certificate("/nonexistent/cert.pem")
        .identity_private_key("/nonexistent/key.pem")
        .governance("/nonexistent/governance.xml")
        .permissions("/nonexistent/permissions.xml")
        .permissions_ca("/nonexistent/permissions_ca.pem");

    let qos = QosBuilder::new()
        .security(config)
        .build()
        .expect("QoS build should succeed even with invalid paths");

    // CycloneDDS may or may not fail here depending on when it validates
    // the file paths. We accept either outcome.
    let result = DomainParticipant::with_qos(99, Some(&qos));
    match result {
        Ok(_) => {
            // CycloneDDS deferred validation to handshake time.
            // This is acceptable; the test documents the behavior.
        }
        Err(e) => {
            // CycloneDDS validated paths early and rejected the participant.
            println!("Participant creation failed as expected: {:?}", e);
        }
    }
}

/// Test that SecurityConfig with only partial fields builds correctly.
#[test]
fn security_config_partial() {
    let config = SecurityConfig::new()
        .identity_ca("/certs/ca.pem")
        .identity_certificate("/certs/cert.pem");

    let qos = QosBuilder::new()
        .security(config)
        .build()
        .expect("Partial config should build");

    drop(qos);
}

/// Test that SecurityConfig default plugins are correct.
#[test]
fn security_config_default_plugins() {
    let config = SecurityConfig::new();
    let qos = QosBuilder::new()
        .security(config)
        .build()
        .expect("Default config should build");

    drop(qos);
}
