//! CDR serialization and deserialization for DDS types.
//!
//! This module provides:
//! - [`CdrSerializer`] / [`CdrDeserializer`] for serializing and deserializing
//!   DDS samples to/from CDR byte streams using the CycloneDDS CDR stream API.
//! - [`CdrSample`] for raw CDR data received via `read_cdr` / `take_cdr`.
//! - [`CdrEncoding`] for selecting XCDR version.

use crate::{DdsError, DdsResult, DdsType};
use cyclonedds_rust_sys::*;
use std::ffi::c_void;
use std::marker::PhantomData;

// ---------------------------------------------------------------------------
// CdrEncoding
// ---------------------------------------------------------------------------

/// CDR encoding version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CdrEncoding {
    /// XCDR1 (CDR version 1, used for plain IDL types).
    Xcdr1,
    /// XCDR2 (CDR version 2, required for appendable/mutable types).
    Xcdr2,
}

impl CdrEncoding {
    fn as_xcdr_version(self) -> u32 {
        match self {
            Self::Xcdr1 => 1,
            Self::Xcdr2 => 2,
        }
    }
}

impl Default for CdrEncoding {
    fn default() -> Self {
        Self::Xcdr1
    }
}

// ---------------------------------------------------------------------------
// CdrSample  (used by read_cdr / take_cdr)
// ---------------------------------------------------------------------------

/// A raw CDR sample obtained from a `DataReader` via `read_cdr` / `take_cdr`.
///
/// Contains the serialized CDR bytes and the associated sample info.
pub struct CdrSample {
    /// Raw CDR bytes (including the encoding header).
    pub data: Vec<u8>,
    /// Sample metadata provided by DDS.
    pub info: dds_sample_info_t,
}

// ---------------------------------------------------------------------------
// CdrSerializer
// ---------------------------------------------------------------------------

/// Serializer that writes a `DdsType` sample into a CDR byte stream.
///
/// Internally uses the CycloneDDS CDR stream writer (`dds_stream_write_sample`).
pub struct CdrSerializer<'a, T: DdsType> {
    _marker: PhantomData<&'a T>,
}

impl<'a, T: DdsType> CdrSerializer<'a, T> {
    /// Serialize `sample` to CDR bytes using the given encoding.
    ///
    /// Builds a temporary `dds_cdrstream_desc` from the type's ops/keys/flagset,
    /// writes the sample into an output stream, and returns the resulting bytes.
    pub fn serialize(
        sample: &T,
        encoding: CdrEncoding,
    ) -> DdsResult<Vec<u8>> {
        let desc = CdrStreamDesc::new::<T>()?;

        unsafe {
            let mut os: dds_ostream_t = std::mem::zeroed();
            dds_ostream_init(
                &mut os,
                &dds_cdrstream_default_allocator,
                0,
                encoding.as_xcdr_version(),
            );

            let mut arena = crate::write_arena::WriteArena::new();
            let data_ptr = sample.write_to_native(&mut arena)?;

            let ok = dds_stream_write_sample(
                &mut os,
                &dds_cdrstream_default_allocator,
                data_ptr,
                desc.as_ptr(),
            );

            let bytes = if ok {
                let len = os.m_index as usize;
                let mut buf = vec![0u8; len];
                std::ptr::copy_nonoverlapping(os.m_buffer, buf.as_mut_ptr(), len);
                Ok(buf)
            } else {
                Err(DdsError::Unsupported("CDR serialization failed".into()))
            };

            dds_ostream_fini(&mut os, &dds_cdrstream_default_allocator);
            bytes
        }
    }

    /// Serialize only the key fields of `sample`.
    pub fn serialize_key(
        sample: &T,
        encoding: CdrEncoding,
    ) -> DdsResult<Vec<u8>> {
        let desc = CdrStreamDesc::new::<T>()?;

        unsafe {
            let mut os: dds_ostream_t = std::mem::zeroed();
            dds_ostream_init(
                &mut os,
                &dds_cdrstream_default_allocator,
                0,
                encoding.as_xcdr_version(),
            );

            let mut arena = crate::write_arena::WriteArena::new();
            let data_ptr = sample.write_to_native(&mut arena)?;

            dds_stream_write_key(
                &mut os,
                cyclonedds_rust_sys::dds_cdr_key_serialization_kind_DDS_CDR_KEY_SERIALIZATION_SAMPLE,
                &dds_cdrstream_default_allocator,
                data_ptr as *const _,
                desc.as_ptr(),
            );

            let len = os.m_index as usize;
            let mut buf = vec![0u8; len];
            std::ptr::copy_nonoverlapping(os.m_buffer, buf.as_mut_ptr(), len);

            dds_ostream_fini(&mut os, &dds_cdrstream_default_allocator);
            Ok(buf)
        }
    }
}

// ---------------------------------------------------------------------------
// CdrDeserializer
// ---------------------------------------------------------------------------

/// Deserializer that reads a `DdsType` sample from CDR bytes.
///
/// Internally uses the CycloneDDS CDR stream reader (`dds_stream_read_sample`).
pub struct CdrDeserializer<T: DdsType> {
    _marker: PhantomData<T>,
}

impl<T: DdsType> CdrDeserializer<T> {
    /// Deserialize a sample from CDR bytes using the given encoding.
    ///
    /// Allocates a zero-initialized buffer of the type's size, reads the CDR
    /// stream into it, then uses `clone_out` to produce an owned value.
    pub fn deserialize(data: &[u8], encoding: CdrEncoding) -> DdsResult<T> {
        let desc = CdrStreamDesc::new::<T>()?;

        unsafe {
            let mut is: dds_istream_t = std::mem::zeroed();
            dds_istream_init(
                &mut is,
                data.len() as u32,
                data.as_ptr() as *const c_void,
                encoding.as_xcdr_version(),
            );

            let size = T::descriptor_size() as usize;
            let align = std::cmp::max(T::descriptor_align() as usize, 1);
            let layout = std::alloc::Layout::from_size_align(size, align)
                .map_err(|_| DdsError::BadParameter("invalid type layout".into()))?;
            let buf = std::alloc::alloc_zeroed(layout);
            if buf.is_null() {
                return Err(DdsError::OutOfMemory);
            }

            dds_stream_read_sample(
                &mut is,
                buf as *mut c_void,
                &dds_cdrstream_default_allocator,
                desc.as_ptr(),
            );

            let result = T::clone_out(buf as *const T);

            // Free any dynamically-allocated members that the read may have created.
            dds_stream_free_sample(
                buf as *mut c_void,
                &dds_cdrstream_default_allocator,
                desc.ops_ptr(),
            );
            std::alloc::dealloc(buf, layout);

            dds_istream_fini(&mut is);
            Ok(result)
        }
    }

    /// Deserialize only the key portion from CDR bytes.
    pub fn deserialize_key(data: &[u8], encoding: CdrEncoding) -> DdsResult<T> {
        let desc = CdrStreamDesc::new::<T>()?;

        unsafe {
            let mut is: dds_istream_t = std::mem::zeroed();
            dds_istream_init(
                &mut is,
                data.len() as u32,
                data.as_ptr() as *const c_void,
                encoding.as_xcdr_version(),
            );

            let size = T::descriptor_size() as usize;
            let align = std::cmp::max(T::descriptor_align() as usize, 1);
            let layout = std::alloc::Layout::from_size_align(size, align)
                .map_err(|_| DdsError::BadParameter("invalid type layout".into()))?;
            let buf = std::alloc::alloc_zeroed(layout);
            if buf.is_null() {
                return Err(DdsError::OutOfMemory);
            }

            dds_stream_read_key(
                &mut is,
                buf as *mut ::std::ffi::c_char,
                &dds_cdrstream_default_allocator,
                desc.as_ptr(),
            );

            let result = T::clone_out(buf as *const T);

            dds_stream_free_sample(
                buf as *mut ::std::ffi::c_void,
                &dds_cdrstream_default_allocator,
                desc.ops_ptr(),
            );
            std::alloc::dealloc(buf, layout);

            dds_istream_fini(&mut is);
            Ok(result)
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: build a dds_cdrstream_desc from DdsType ops/keys
// ---------------------------------------------------------------------------

/// RAII wrapper around `dds_cdrstream_desc` that is initialised from a type's
/// ops array and cleaned up on drop.
struct CdrStreamDesc {
    desc: dds_cdrstream_desc,
    _ops: Vec<u32>,
    _key_names: Vec<std::ffi::CString>,
    _keys: Vec<dds_key_descriptor>,
}

impl CdrStreamDesc {
    /// Build a new CDR stream descriptor for the given `DdsType`.
    fn new<T: DdsType>() -> DdsResult<Self> {
        // Build ops array including OP_RTS and KOF entries, same as Topic::with_qos
        let mut ops = T::ops();
        if ops.last().copied() != Some(crate::topic::OP_RTS) {
            ops.push(crate::topic::OP_RTS);
        }

        let key_defs = T::keys();
        let key_names: Vec<std::ffi::CString> = key_defs
            .iter()
            .map(|k| std::ffi::CString::new(k.name.as_str()).unwrap())
            .collect();
        let mut keys: Vec<dds_key_descriptor> = Vec::with_capacity(key_defs.len());
        for (i, kd) in key_defs.iter().enumerate() {
            let offset = ops.len() as u32;
            ops.push(crate::topic::OP_KOF | (kd.ops_path.len() as u32));
            ops.extend(kd.ops_path.iter().copied());
            keys.push(dds_key_descriptor {
                m_name: key_names[i].as_ptr(),
                m_offset: offset,
                m_idx: i as u32,
            });
        }

        let post_key_ops = T::post_key_ops();
        if !post_key_ops.is_empty() {
            ops.extend(post_key_ops);
        }

        unsafe {
            let mut desc: dds_cdrstream_desc = std::mem::zeroed();
            dds_cdrstream_desc_init(
                &mut desc,
                &dds_cdrstream_default_allocator,
                T::descriptor_size(),
                T::descriptor_align(),
                T::flagset(),
                ops.as_ptr(),
                if keys.is_empty() {
                    std::ptr::null()
                } else {
                    keys.as_ptr()
                },
                keys.len() as u32,
            );

            Ok(Self {
                desc,
                _ops: ops,
                _key_names: key_names,
                _keys: keys,
            })
        }
    }

    fn as_ptr(&self) -> *const dds_cdrstream_desc {
        &self.desc
    }

    /// Returns the raw ops pointer for use with functions like `dds_stream_free_sample`.
    fn ops_ptr(&self) -> *const u32 {
        self.desc.ops.ops as *const u32
    }
}

impl Drop for CdrStreamDesc {
    fn drop(&mut self) {
        unsafe {
            dds_cdrstream_desc_fini(&mut self.desc, &dds_cdrstream_default_allocator);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Minimal type for testing serialization round-trip.
    // Uses the DdsType derive macro if available; otherwise we hand-roll it.
    #[derive(Debug, PartialEq, Clone)]
    struct Point {
        x: i32,
        y: i32,
    }

    impl DdsType for Point {
        fn type_name() -> &'static str {
            "test::Point"
        }
        fn ops() -> Vec<u32> {
            use crate::topic::*;
            let mut ops = Vec::new();
            ops.extend(adr(TYPE_4BY, 0)); // x at offset 0
            ops.extend(adr(TYPE_4BY, 4)); // y at offset 4
            ops
        }
        fn descriptor_size() -> u32 {
            8
        }
        fn descriptor_align() -> u32 {
            4
        }
    }

    #[test]
    fn cdr_encoding_default_is_xcdr1() {
        assert_eq!(CdrEncoding::default(), CdrEncoding::Xcdr1);
        assert_eq!(CdrEncoding::Xcdr1.as_xcdr_version(), 1);
        assert_eq!(CdrEncoding::Xcdr2.as_xcdr_version(), 2);
    }
}
