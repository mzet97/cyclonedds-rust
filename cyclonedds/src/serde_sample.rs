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

use crate::{write_arena::WriteArena, DdsResult, DdsType};
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

/// Wire representation of a [`SerdeSample`]: what the `ADR | SEQ | 1BY` op in
/// [`SerdeSample::ops`] actually describes.
///
/// This exists because the previous implementation used `Native = Self` and
/// handed the Rust `SerdeSample` straight to CycloneDDS. That is unsound three
/// ways over: the ops declare a DDS byte sequence, so the C side reads the
/// buffer as `dds_sequence_t` (`{u32 _maximum, u32 _length, *mut u8 _buffer,
/// bool _release}`) while the memory actually held a `Vec<u8>`; `Vec`'s field
/// order is `repr(Rust)` and explicitly not guaranteed, so even the sizes
/// matching was luck; and `clone_out` did `ptr::read`, taking Rust ownership of
/// a buffer allocated by `dds_alloc`, which Rust would then free with its own
/// allocator.
///
/// `DdsSequence<u8>` is the layout CycloneDDS expects, and the conversions below
/// mirror exactly what `#[derive(DdsTypeDerive)]` generates for a `Vec<u8>`
/// field.
#[repr(C)]
pub struct SerdeSampleNative {
    payload: crate::DdsSequence<u8>,
}

impl<T: serde::Serialize + for<'de> serde::Deserialize<'de> + Send + 'static> DdsType
    for SerdeSample<T>
{
    type Native = SerdeSampleNative;

    fn type_name() -> &'static str {
        // NOTE: this is the same string for every `T`. `stringify!(T)` expands
        // to the literal "T", not the substituted type, so `SerdeSample<A>` and
        // `SerdeSample<B>` announce the same DDS type name and will match each
        // other on the wire. Fixing it needs a name that is stable across peers
        // and across compilations — `std::any::type_name` is neither — so it is
        // left as-is and recorded in docs/soundness-backlog.md rather than
        // guessed at here.
        "SerdeSample"
    }

    fn ops() -> Vec<u32> {
        // Variable-length byte sequence: [ADR | SEQ | 1BY, offset, OP_RTS]
        use crate::topic::{OP_ADR, OP_RTS, SUBTYPE_1BY, TYPE_SEQ};
        vec![
            OP_ADR | TYPE_SEQ | SUBTYPE_1BY,
            std::mem::offset_of!(SerdeSampleNative, payload) as u32,
            OP_RTS,
        ]
    }

    unsafe fn clone_out(ptr: *const Self) -> DdsResult<Self> {
        let native = &*(ptr as *const SerdeSampleNative);
        Ok(SerdeSample {
            payload: native.payload.to_vec(),
            _marker: std::marker::PhantomData,
        })
    }

    fn write_to_native<'a>(&'a self, arena: &'a mut WriteArena) -> DdsResult<*const c_void> {
        let native = SerdeSampleNative {
            payload: crate::DdsSequence::from_slice(&self.payload)?,
        };
        Ok(arena.hold(native).cast())
    }
}
