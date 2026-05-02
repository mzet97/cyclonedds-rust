//! DDS Security configuration helpers.
//!
//! CycloneDDS implements DDS Security via plugins configured through QoS
//! properties. This module provides a builder-style API for setting up
//! authentication, access control, and cryptography.
//!
//! # Example
//!
//! ```no_run
//! use cyclonedds::{DomainParticipant, QosBuilder, SecurityConfig};
//!
//! let security = SecurityConfig::new()
//!     .identity_ca("certs/identity_ca.pem")
//!     .identity_certificate("certs/cert.pem")
//!     .identity_private_key("certs/key.pem")
//!     .governance("governance.xml")
//!     .permissions("permissions.xml")
//!     .permissions_ca("certs/permissions_ca.pem");
//!
//! let qos = QosBuilder::new()
//!     .security(security)
//!     .build()
//!     .unwrap();
//!
//! let participant = DomainParticipant::with_qos(0, &qos).unwrap();
//! ```

use crate::QosBuilder;

/// Builder for DDS Security configuration.
///
/// Security is enabled by setting specific QoS properties that tell
/// CycloneDDS which plugins to load and where to find certificates,
/// keys, and policy files.
#[derive(Debug, Clone, Default)]
pub struct SecurityConfig {
    identity_ca: Option<String>,
    identity_certificate: Option<String>,
    identity_private_key: Option<String>,
    governance: Option<String>,
    permissions: Option<String>,
    permissions_ca: Option<String>,
    auth_plugin: String,
    access_plugin: String,
    crypto_plugin: String,
}

impl SecurityConfig {
    /// Create a new security config with default plugin names.
    pub fn new() -> Self {
        Self {
            auth_plugin: "dds.sec.auth.builtin.PKI-DH".into(),
            access_plugin: "dds.sec.access.builtin.Access-Permissions".into(),
            crypto_plugin: "dds.sec.crypto.builtin.AES-GCM-GMAC".into(),
            ..Default::default()
        }
    }

    /// Path to the Identity CA certificate (PEM).
    pub fn identity_ca(mut self, path: impl Into<String>) -> Self {
        self.identity_ca = Some(path.into());
        self
    }

    /// Path to the participant identity certificate (PEM).
    pub fn identity_certificate(mut self, path: impl Into<String>) -> Self {
        self.identity_certificate = Some(path.into());
        self
    }

    /// Path to the participant identity private key (PEM).
    pub fn identity_private_key(mut self, path: impl Into<String>) -> Self {
        self.identity_private_key = Some(path.into());
        self
    }

    /// Path to the Governance XML document.
    pub fn governance(mut self, path: impl Into<String>) -> Self {
        self.governance = Some(path.into());
        self
    }

    /// Path to the Permissions XML document.
    pub fn permissions(mut self, path: impl Into<String>) -> Self {
        self.permissions = Some(path.into());
        self
    }

    /// Path to the Permissions CA certificate (PEM).
    pub fn permissions_ca(mut self, path: impl Into<String>) -> Self {
        self.permissions_ca = Some(path.into());
        self
    }

    /// Override the authentication plugin name (default: `dds.sec.auth.builtin.PKI-DH`).
    pub fn auth_plugin(mut self, name: impl Into<String>) -> Self {
        self.auth_plugin = name.into();
        self
    }

    /// Override the access-control plugin name (default: `dds.sec.access.builtin.Access-Permissions`).
    pub fn access_plugin(mut self, name: impl Into<String>) -> Self {
        self.access_plugin = name.into();
        self
    }

    /// Override the cryptography plugin name (default: `dds.sec.crypto.builtin.AES-GCM-GMAC`).
    pub fn crypto_plugin(mut self, name: impl Into<String>) -> Self {
        self.crypto_plugin = name.into();
        self
    }

    /// Validate that all configured certificate and policy files exist
    /// and appear to be valid PEM documents.
    ///
    /// Returns `Ok(())` if all configured files are present and readable,
    /// or `Err` with a description of the first problem found.
    pub fn validate(&self) -> crate::DdsResult<()> {
        let paths = [
            ("identity_ca", self.identity_ca.as_deref()),
            ("identity_certificate", self.identity_certificate.as_deref()),
            ("identity_private_key", self.identity_private_key.as_deref()),
            ("governance", self.governance.as_deref()),
            ("permissions", self.permissions.as_deref()),
            ("permissions_ca", self.permissions_ca.as_deref()),
        ];

        for (name, maybe_path) in &paths {
            if let Some(path) = maybe_path {
                let contents = std::fs::read_to_string(path).map_err(|e| {
                    crate::DdsError::BadParameter(format!(
                        "security {} file '{}' not readable: {}",
                        name, path, e
                    ))
                })?;
                if !contents.contains("-----BEGIN") {
                    return Err(crate::DdsError::BadParameter(format!(
                        "security {} file '{}' does not appear to be a valid PEM document",
                        name, path
                    )));
                }
            }
        }
        Ok(())
    }

    /// Create a new `SecurityConfig` with the same settings.
    ///
    /// This is a semantic wrapper for certificate hot-reload workflows:
    /// after updating certificate files on disk, call `reload()` and then
    /// `validate()` before recreating the `DomainParticipant`.
    pub fn reload(&self) -> Self {
        self.clone()
    }

    /// Apply this security configuration to a [`QosBuilder`].
    pub fn apply_to(self, builder: QosBuilder) -> QosBuilder {
        let mut b = builder
            .property("dds.sec.auth.plugin", &self.auth_plugin)
            .property("dds.sec.access.plugin", &self.access_plugin)
            .property("dds.sec.crypto.plugin", &self.crypto_plugin);

        if let Some(path) = self.identity_ca {
            b = b.property("dds.sec.auth.identity_ca", format!("file:{}", path));
        }
        if let Some(path) = self.identity_certificate {
            b = b.property(
                "dds.sec.auth.identity_certificate",
                format!("file:{}", path),
            );
        }
        if let Some(path) = self.identity_private_key {
            b = b.property("dds.sec.auth.private_key", format!("file:{}", path));
        }
        if let Some(path) = self.governance {
            b = b.property("dds.sec.access.governance", format!("file:{}", path));
        }
        if let Some(path) = self.permissions {
            b = b.property("dds.sec.access.permissions", format!("file:{}", path));
        }
        if let Some(path) = self.permissions_ca {
            b = b.property("dds.sec.access.permissions_ca", format!("file:{}", path));
        }

        b
    }
}

impl QosBuilder {
    /// Convenience: apply a [`SecurityConfig`] directly.
    pub fn security(self, config: SecurityConfig) -> Self {
        config.apply_to(self)
    }
}
