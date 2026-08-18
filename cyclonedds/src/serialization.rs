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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CdrEncoding {
    /// XCDR1 (CDR version 1, used for plain IDL types).
    #[default]
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

impl<T: DdsType> CdrSerializer<'_, T> {
    /// Serialize `sample` to CDR bytes using the given encoding.
    ///
    /// Builds a temporary `dds_cdrstream_desc` from the type's ops/keys/flagset,
    /// writes the sample into an output stream, and returns the resulting bytes.
    pub fn serialize(sample: &T, encoding: CdrEncoding) -> DdsResult<Vec<u8>> {
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
    pub fn serialize_key(sample: &T, encoding: CdrEncoding) -> DdsResult<Vec<u8>> {
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

    /// Serialize `sample` into a pre-allocated buffer, returning the number
    /// of bytes written.
    ///
    /// This avoids the intermediate allocation performed by `serialize`.
    /// If `buf` is too small, an error is returned.
    pub fn serialize_to_buffer(
        sample: &T,
        buf: &mut [u8],
        encoding: CdrEncoding,
    ) -> DdsResult<usize> {
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

            if !ok {
                dds_ostream_fini(&mut os, &dds_cdrstream_default_allocator);
                return Err(DdsError::Unsupported("CDR serialization failed".into()));
            }

            let len = os.m_index as usize;
            if len > buf.len() {
                dds_ostream_fini(&mut os, &dds_cdrstream_default_allocator);
                return Err(DdsError::BadParameter(format!(
                    "buffer too small: needed {} bytes, got {}",
                    len,
                    buf.len()
                )));
            }

            std::ptr::copy_nonoverlapping(os.m_buffer, buf.as_mut_ptr(), len);
            dds_ostream_fini(&mut os, &dds_cdrstream_default_allocator);
            Ok(len)
        }
    }

    /// Serialize only the key fields into a pre-allocated buffer.
    pub fn serialize_key_to_buffer(
        sample: &T,
        buf: &mut [u8],
        encoding: CdrEncoding,
    ) -> DdsResult<usize> {
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
            if len > buf.len() {
                dds_ostream_fini(&mut os, &dds_cdrstream_default_allocator);
                return Err(DdsError::BadParameter(format!(
                    "buffer too small: needed {} bytes, got {}",
                    len,
                    buf.len()
                )));
            }

            std::ptr::copy_nonoverlapping(os.m_buffer, buf.as_mut_ptr(), len);
            dds_ostream_fini(&mut os, &dds_cdrstream_default_allocator);
            Ok(len)
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
    /// Validate `data` against `T`'s descriptor, returning a normalized copy.
    ///
    /// `dds_istream_init`'s contract is explicit that the buffer "must contain
    /// well-formed CDR data in native endianness. Use `dds_stream_normalize` to
    /// verify well-formedness" (`dds/cdr/dds_cdrstream.h`). `deserialize` and
    /// `deserialize_key` skipped that step and handed caller-supplied bytes
    /// straight to `dds_stream_read_sample`, which trusts every length prefix it
    /// reads. They are safe functions taking an arbitrary `&[u8]`, so any caller
    /// could reach it — including anyone feeding them the bytes that
    /// `read_cdr`/`take_cdr` returns, which come off the network.
    ///
    /// Normalization byte-swaps in place, so it needs its own mutable copy;
    /// `bswap` is false because this pair's contract is native endianness, which
    /// is what `CdrSerializer` emits.
    fn normalized(
        data: &[u8],
        encoding: CdrEncoding,
        desc: &CdrStreamDesc,
        just_key: bool,
    ) -> DdsResult<Vec<u8>> {
        let mut buf = data.to_vec();
        let mut actual_size: u32 = 0;
        let ok = unsafe {
            dds_stream_normalize(
                buf.as_mut_ptr() as *mut c_void,
                buf.len() as u32,
                false,
                encoding.as_xcdr_version(),
                desc.as_ptr(),
                just_key,
                &mut actual_size,
            )
        };
        if !ok {
            return Err(DdsError::BadParameter(
                "malformed CDR data for this type".into(),
            ));
        }
        // `actual_size <= size`; anything past it is not part of the sample.
        buf.truncate(actual_size as usize);
        Ok(buf)
    }

    /// Deserialize a sample from CDR bytes using the given encoding.
    ///
    /// The input is validated against `T`'s descriptor before it is read; bytes
    /// that do not describe a well-formed sample of `T` are rejected with
    /// [`DdsError::BadParameter`] rather than followed.
    ///
    /// Allocates a zero-initialized buffer of the type's size, reads the CDR
    /// stream into it, then uses `clone_out` to produce an owned value.
    pub fn deserialize(data: &[u8], encoding: CdrEncoding) -> DdsResult<T> {
        let desc = CdrStreamDesc::new::<T>()?;
        let data = Self::normalized(data, encoding, &desc, false)?;
        let data = data.as_slice();

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

            // Decode first, then clean up unconditionally: applying `?` before
            // the frees below would leak the native sample and the stream.
            let decoded = T::clone_out(buf as *const T);

            // Free any dynamically-allocated members that the read may have created.
            dds_stream_free_sample(
                buf as *mut c_void,
                &dds_cdrstream_default_allocator,
                desc.ops_ptr(),
            );
            std::alloc::dealloc(buf, layout);

            dds_istream_fini(&mut is);
            decoded
        }
    }

    /// Deserialize only the key portion from CDR bytes.
    ///
    /// Validated the same way as [`CdrDeserializer::deserialize`], against the
    /// key subset of the descriptor.
    pub fn deserialize_key(data: &[u8], encoding: CdrEncoding) -> DdsResult<T> {
        let desc = CdrStreamDesc::new::<T>()?;
        let data = Self::normalized(data, encoding, &desc, true)?;
        let data = data.as_slice();

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

            // Decode first, then clean up unconditionally: applying `?` before
            // the frees below would leak the native sample and the stream.
            let decoded = T::clone_out(buf as *const T);

            dds_stream_free_sample(
                buf as *mut ::std::ffi::c_void,
                &dds_cdrstream_default_allocator,
                desc.ops_ptr(),
            );
            std::alloc::dealloc(buf, layout);

            dds_istream_fini(&mut is);
            decoded
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
    #[allow(dead_code)]
    struct Point {
        x: i32,
        y: i32,
    }

    unsafe impl DdsType for Point {
        type Native = Self;

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
