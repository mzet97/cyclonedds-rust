//! Safe Rust wrapper for CycloneDDS
//!
//! This crate provides a safe, idiomatic Rust API for CycloneDDS.
//! It wraps the low-level FFI bindings from `cyclonedds-sys` with RAII types
//! and proper error handling.

pub mod error;
pub mod participant;
pub mod topic;
pub mod publisher;
pub mod subscriber;
pub mod writer;
pub mod reader;

pub use error::{DdsError, DdsResult};
pub use participant::DomainParticipant;
pub use topic::Topic;
pub use publisher::Publisher;
pub use subscriber::Subscriber;
pub use writer::DataWriter;
pub use reader::DataReader;
