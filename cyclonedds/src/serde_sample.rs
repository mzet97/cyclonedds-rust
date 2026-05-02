//! Serde-based DDS sample wrapper.
//!
//! When the `serde` feature is enabled, [`SerdeSample<T>`] provides a
//! [`DdsType`] implementation for any `T: Serialize + DeserializeOwned`.
//! The sample is serialized to a compact binary format using ` postcard`
//! and transmitted as a variable-length byte sequence.
//!
//! # Interoperability Note
//!
//! `SerdeSample` uses ` postcard` encoding, **not** OMG CDR/XCDR.
//! This means it can only communicate with other Rust nodes using the
//! same wrapper, or with a bridge that translates the format.
//!
//! # Example
//!
//! ```no_run
//! use cyclonedds::SerdeSample;
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, Clone, Debug)]
//! struct MyMessage {
//!     id: u32,
//!     text: String,
//! }
//!
//! let participant = cyclonedds::DomainParticipant::new(0).unwrap();
//! let topic = participant
//!     .create_topic::<SerdeSample<MyMessage>>("MyTopic")
//!     .unwrap();
//! ```

use crate::{DdsResult, DdsType, write_arena::WriteArena};
use std::ffi::c_void;

/// A DDS sample that wraps any serde-compatible type.
///
/// `SerdeSample<T>` stores the serialized payload internally and
/// implements [`DdsType`] so it can be used with [`Topic`](crate::Topic),
/// [`DataWriter`](crate::DataWriter) and [`DataReader`](crate::DataReader).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerdeSample<T> {
    payload: Vec<u8>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> SerdeSample<T> {
    /// Create a `SerdeSample` from a serializable value.
    pub fn new(value: &T) -> DdsResult<Self>
    where
        T: serde::Serialize,
    {
        let payload = postcard::to_vec::<T, 4096>(value).map_err(|e| {
            crate::DdsError::BadParameter(format!("serde serialization failed: {}", e))
        })?;
        Ok(SerdeSample {
            payload: payload.to_vec(),
            _marker: std::marker::PhantomData,
        })
    }

    /// Deserialize the inner value.
    pub fn deserialize(&self) -> DdsResult<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        postcard::from_bytes(&self.payload).map_err(|e| {
            crate::DdsError::BadParameter(format!("serde deserialization failed: {}", e))
        })
    }

    /// Returns the raw serialized payload.
    pub fn as_bytes(&self) -> &[u8] {
        &self.payload
    }
}

impl<T: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + 'static> DdsType
    for SerdeSample<T>
{
    fn type_name() -> &'static str {
        concat!("SerdeSample<", stringify!(T), ">")
    }

    fn ops() -> Vec<u32> {
        // Variable-length byte sequence: [ADR | SEQ | 1BY, offset, OP_RTS]
        use crate::topic::{OP_ADR, OP_RTS, TYPE_SEQ, SUBTYPE_1BY};
        vec![
            OP_ADR | TYPE_SEQ | SUBTYPE_1BY,
            0, // offset of payload field
            OP_RTS,
        ]
    }

    fn descriptor_size() -> u32 {
        std::mem::size_of::<Vec<u8>>() as u32
    }

    fn descriptor_align() -> u32 {
        std::mem::align_of::<Vec<u8>>() as u32
    }

    unsafe fn clone_out(ptr: *const Self) -> Self {
        std::ptr::read(ptr)
    }

    fn write_to_native<'a>(
        &'a self,
        _arena: &'a mut WriteArena,
    ) -> DdsResult<*const c_void> {
        Ok(self as *const Self as *const c_void)
    }
}
