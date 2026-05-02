//! Core DDS types for `no_std` environments.
//!
//! This module provides the `DdsType` trait, opcode constants, and helper
//! functions needed to define DDS topic types without requiring the full
//! CycloneDDS C library (which needs `std` and cannot run on embedded targets).
//!
//! When the `no_std` feature is enabled, only this module and a handful of
//! pure-Rust utilities are compiled.
//!
//! # Limitations
//!
//! - No actual DDS networking (no DomainParticipant, Reader, Writer).
//! - `DdsType::clone_out` and `write_to_native` are omitted (they need `std`).
//! - Useful for defining types and generating CDR ops on embedded systems.

#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;
use core::fmt;

// ---------------------------------------------------------------------------
// Opcode constants (mirrored from OMG DDS-XTypes CDR stream spec)
// ---------------------------------------------------------------------------

pub const OP_RTS: u32 = 0x00 << 24;
pub const OP_DLC: u32 = 0x01 << 24;
pub const OP_ADR: u32 = 0x02 << 24;
pub const OP_JEQ4: u32 = 0x05 << 24;
pub const OP_MID: u32 = 0x06 << 24;
pub const OP_KOF: u32 = 0x07 << 24;

pub const VAL_1BY: u32 = 0x01;
pub const VAL_2BY: u32 = 0x02;
pub const VAL_4BY: u32 = 0x04;
pub const VAL_8BY: u32 = 0x05;
pub const VAL_STR: u32 = 0x06;
pub const VAL_BST: u32 = 0x07;
pub const VAL_SEQ: u32 = 0x08;
pub const VAL_BSQ: u32 = 0x09;
pub const VAL_ARR: u32 = 0x0a;

pub const TYPE_1BY: u32 = VAL_1BY << 16;
pub const TYPE_2BY: u32 = VAL_2BY << 16;
pub const TYPE_4BY: u32 = VAL_4BY << 16;
pub const TYPE_8BY: u32 = VAL_8BY << 16;
pub const TYPE_STR: u32 = VAL_STR << 16;
pub const TYPE_BST: u32 = VAL_BST << 16;
pub const TYPE_SEQ: u32 = VAL_SEQ << 16;
pub const TYPE_BSQ: u32 = VAL_BSQ << 16;
pub const TYPE_ARR: u32 = VAL_ARR << 16;
pub const TYPE_ENU: u32 = 0x0b << 16;
pub const TYPE_EXT: u32 = 0x0c << 16;
pub const TYPE_UNI: u32 = 0x0d << 16;

pub const SUBTYPE_1BY: u32 = 0x01 << 16;
pub const SUBTYPE_2BY: u32 = 0x02 << 16;
pub const SUBTYPE_4BY: u32 = 0x04 << 16;
pub const SUBTYPE_8BY: u32 = 0x05 << 16;
pub const SUBTYPE_STR: u32 = 0x06 << 16;
pub const SUBTYPE_BST: u32 = 0x07 << 16;
pub const SUBTYPE_SEQ: u32 = 0x08 << 16;
pub const SUBTYPE_BSQ: u32 = 0x09 << 16;
pub const SUBTYPE_STU: u32 = 0x0a << 16;
pub const SUBTYPE_ENU: u32 = 0x0b << 16;

pub const OP_FLAG_SZ_SHIFT: u32 = 16;
pub const DDS_OP_MASK_CONST: u32 = 0x00ffffff;
pub const DDS_OP_TYPE_MASK_CONST: u32 = 0x3f000000;
pub const DDS_OP_SUBTYPE_MASK_CONST: u32 = 0x3f0000;

pub const OP_FLAG_KEY: u32 = 1 << 0;
pub const OP_FLAG_FP: u32 = 1 << 1;
pub const OP_FLAG_SGN: u32 = 1 << 2;
pub const OP_FLAG_EXT: u32 = 1 << 3;
pub const OP_FLAG_MU: u32 = 1 << 4;
pub const OP_FLAG_OPT: u32 = 1 << 5;

// ---------------------------------------------------------------------------
// Helper functions for building ops arrays
// ---------------------------------------------------------------------------

/// Build an ADR opcode for a simple field.
/// `typecode` should be one of the `TYPE_*` constants.
/// `offset` is the byte offset of the field within the struct.
pub const fn adr(typecode: u32, offset: u32) -> [u32; 2] {
    [OP_ADR | typecode | (offset & 0xffff), offset >> 16]
}

/// Build an ADR opcode for a bounded string field.
/// `offset` is the byte offset; `max_len` is the maximum string length.
pub const fn adr_bst(offset: u32, max_len: u32) -> [u32; 3] {
    [OP_ADR | TYPE_BST | (offset & 0xffff), offset >> 16, max_len]
}

/// Build an ADR opcode for a keyed field.
pub const fn adr_key(typecode: u32, offset: u32) -> [u32; 2] {
    [OP_ADR | typecode | OP_FLAG_KEY | (offset & 0xffff), offset >> 16]
}

/// Rebase (resize) an ops array by replacing the low 16 bits of each element
/// with the corresponding value from `sizes`.
pub fn rebase_ops(ops: &mut [u32], sizes: &[u32]) {
    assert_eq!(ops.len(), sizes.len());
    for (op, size) in ops.iter_mut().zip(sizes.iter()) {
        *op = (*op & !0xffff) | (size & 0xffff);
    }
}

// ---------------------------------------------------------------------------
// Key descriptor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct KeyDescriptor {
    pub name: &'static str,
    pub ops_path: Vec<u32>,
}

// ---------------------------------------------------------------------------
// DdsType trait (no_std compatible subset)
// ---------------------------------------------------------------------------

/// Trait that DDS topic types must implement.
///
/// In `no_std` mode this is a pure-Rust trait without FFI methods.
pub trait DdsType: Sized + Send + 'static {
    fn type_name() -> &'static str;
    fn ops() -> Vec<u32>;
    fn descriptor_size() -> u32 {
        core::mem::size_of::<Self>() as u32
    }
    fn descriptor_align() -> u32 {
        core::mem::align_of::<Self>() as u32
    }
    fn key_count() -> usize {
        0
    }
    fn keys() -> Vec<KeyDescriptor> {
        Vec::new()
    }
    fn flagset() -> u32 {
        0
    }
    fn post_key_ops() -> Vec<u32> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// Enum / Union / Bitmask traits
// ---------------------------------------------------------------------------

pub trait DdsEnumType: Sized + Copy + Send + 'static {
    fn max_discriminant() -> u32;
    fn as_u32(self) -> u32;
    fn from_u32(value: u32) -> Option<Self>;
}

pub trait DdsUnionType: Sized + Send + 'static {
    type Discriminant: DdsEnumType;
    fn discriminant(&self) -> Self::Discriminant;
}

pub trait DdsBitmaskType: Sized + Copy + Send + 'static {
    fn as_u32(self) -> u32;
    fn from_u32(value: u32) -> Self;
}

// ---------------------------------------------------------------------------
// Discriminant type marker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscriminantType {
    U8,
    U16,
    U32,
}

impl fmt::Display for DiscriminantType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiscriminantType::U8 => write!(f, "u8"),
            DiscriminantType::U16 => write!(f, "u16"),
            DiscriminantType::U32 => write!(f, "u32"),
        }
    }
}
