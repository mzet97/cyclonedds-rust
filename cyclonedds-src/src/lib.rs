//! Source distribution of Eclipse CycloneDDS C library.
//!
//! This crate bundles the CycloneDDS C source code so that downstream
//! `cyclonedds-rust-sys` can build it from source when no system library
//! is available.
//!
//! # Usage
//!
//! Typically used as a `build-dependency` in `cyclonedds-rust-sys`:
//!
//! ```toml
//! [build-dependencies]
//! cyclonedds-src = "0.1"
//! ```
//!
//! Then in `build.rs`:
//!
//! ```no_run
//! let src = cyclonedds_src::source_dir();
//! // build with cmake
//! ```

use std::path::PathBuf;

/// Return the directory containing the CycloneDDS C source tree.
///
/// This can be passed to `cmake::Config` or used directly in a build script.
pub fn source_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/cyclonedds")
}

/// Return the directory where the C headers are located.
pub fn include_dir() -> PathBuf {
    source_dir().join("src/core/ddsc/include")
}

/// Return the directory where the generated config header is expected.
pub fn build_include_dir() -> PathBuf {
    // This is set by the downstream build script after running cmake.
    PathBuf::from(std::env::var_os("OUT_DIR").unwrap_or_default())
        .join("cyclonedds-build")
        .join("src")
        .join("core")
}
