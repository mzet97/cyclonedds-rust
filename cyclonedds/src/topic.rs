use crate::{
    entity::DdsEntity, write_arena::WriteArena, xtypes::TopicDescriptor, DdsError, DdsResult, Qos,
};
use cyclonedds_rust_sys::*;
use std::ffi::c_void;
use std::ffi::CString;
use std::marker::PhantomData;
use std::rc::Rc;

/// A topic without compile-time type information.
pub struct UntypedTopic {
    entity: dds_entity_t,
}

impl UntypedTopic {
    pub(crate) fn from_entity(entity: dds_entity_t) -> Self {
        Self { entity }
    }

    pub fn from_descriptor(
        participant: dds_entity_t,
        name: &str,
        descriptor: &TopicDescriptor,
    ) -> DdsResult<Self> {
        Self::from_descriptor_with_qos(participant, name, descriptor, None)
    }

    pub fn from_descriptor_with_qos(
        participant: dds_entity_t,
        name: &str,
        descriptor: &TopicDescriptor,
        qos: Option<&Qos>,
    ) -> DdsResult<Self> {
        let topic_name = CString::new(name)
            .map_err(|_| DdsError::BadParameter("topic name contains null".into()))?;
        unsafe {
            let handle = dds_create_topic(
                participant,
                descriptor.as_ptr(),
                topic_name.as_ptr(),
                qos.map_or(std::ptr::null(), |q| q.as_ptr()),
                std::ptr::null(),
            );
            crate::error::check_entity(handle)?;
            Ok(Self { entity: handle })
        }
    }
}

impl DdsEntity for UntypedTopic {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl Drop for UntypedTopic {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}

pub struct Topic<T> {
    entity: dds_entity_t,
    _holder: Rc<DescriptorHolder>,
    _marker: PhantomData<T>,
}

struct DescriptorHolder {
    _ops: Vec<u32>,
    _typename: CString,
    _key_names: Vec<CString>,
    _keys: Vec<dds_key_descriptor>,
    _meta: CString,
}

pub struct TopicKeyDescriptor {
    pub name: String,
    pub offset: u32,
    pub index: u32,
}

pub const OP_RTS: u32 = dds_stream_opcode_DDS_OP_RTS;
pub const OP_DLC: u32 = dds_stream_opcode_DDS_OP_DLC;
pub const OP_ADR: u32 = dds_stream_opcode_DDS_OP_ADR;
pub const OP_JEQ4: u32 = dds_stream_opcode_DDS_OP_JEQ4;

pub const VAL_1BY: u32 = dds_stream_typecode_DDS_OP_VAL_1BY;
pub const VAL_2BY: u32 = dds_stream_typecode_DDS_OP_VAL_2BY;
pub const VAL_4BY: u32 = dds_stream_typecode_DDS_OP_VAL_4BY;
pub const VAL_8BY: u32 = dds_stream_typecode_DDS_OP_VAL_8BY;
pub const VAL_STR: u32 = dds_stream_typecode_DDS_OP_VAL_STR;
pub const VAL_BST: u32 = dds_stream_typecode_DDS_OP_VAL_BST;
pub const VAL_SEQ: u32 = dds_stream_typecode_DDS_OP_VAL_SEQ;
pub const VAL_BSQ: u32 = dds_stream_typecode_DDS_OP_VAL_BSQ;
pub const VAL_ARR: u32 = dds_stream_typecode_DDS_OP_VAL_ARR;

pub const TYPE_1BY: u32 = VAL_1BY << 16;
pub const TYPE_2BY: u32 = VAL_2BY << 16;
pub const TYPE_4BY: u32 = VAL_4BY << 16;
pub const TYPE_8BY: u32 = VAL_8BY << 16;
pub const TYPE_STR: u32 = VAL_STR << 16;
pub const TYPE_BST: u32 = VAL_BST << 16;
pub const TYPE_SEQ: u32 = VAL_SEQ << 16;
pub const TYPE_BSQ: u32 = VAL_BSQ << 16;
pub const TYPE_ARR: u32 = VAL_ARR << 16;
pub const TYPE_ENU: u32 = dds_stream_typecode_primary_DDS_OP_TYPE_ENU;
pub const TYPE_EXT: u32 = dds_stream_typecode_primary_DDS_OP_TYPE_EXT;
pub const TYPE_UNI: u32 = dds_stream_typecode_primary_DDS_OP_TYPE_UNI;

pub const SUBTYPE_1BY: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_1BY;
pub const SUBTYPE_2BY: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_2BY;
pub const SUBTYPE_4BY: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_4BY;
pub const SUBTYPE_8BY: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_8BY;
pub const SUBTYPE_STR: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_STR;
pub const SUBTYPE_BST: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_BST;
pub const SUBTYPE_SEQ: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_SEQ;
pub const SUBTYPE_BSQ: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_BSQ;
pub const SUBTYPE_STU: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_STU;
pub const SUBTYPE_ENU: u32 = dds_stream_typecode_subtype_DDS_OP_SUBTYPE_ENU;
pub const OP_FLAG_SZ_SHIFT: u32 = DDS_OP_FLAG_SZ_SHIFT;
pub const DDS_OP_MASK_CONST: u32 = DDS_OP_MASK;
pub const DDS_OP_TYPE_MASK_CONST: u32 = DDS_OP_TYPE_MASK;
pub const DDS_OP_SUBTYPE_MASK_CONST: u32 = DDS_OP_SUBTYPE_MASK;

/// ADR flags (low 8 bits of the ADR opcode)
pub const OP_FLAG_KEY: u32 = 1u32 << 0;
pub const OP_FLAG_FP: u32 = DDS_OP_FLAG_FP;
pub const OP_FLAG_SGN: u32 = 1u32 << 2;
pub const OP_FLAG_EXT: u32 = DDS_OP_FLAG_EXT;
pub const OP_FLAG_MU: u32 = DDS_OP_FLAG_MU;
pub const OP_FLAG_OPT: u32 = DDS_OP_FLAG_OPT;

/// Key Offset Format opcode.
/// Format: `[KOF | n, adr_ops_index_0, ..., adr_ops_index_(n-1)]`
/// where n = number of ADR opcode indices that contribute to this key path.
/// For a flat key field (single member), n=1: `[KOF | 1, adr_index]`.
/// KOF entries are placed AFTER OP_RTS in the ops array.
pub const OP_KOF: u32 = 0x07 << 24;
pub const OP_MID: u32 = dds_stream_opcode_DDS_OP_MID;

pub trait DdsType: Sized + Send + 'static {
    fn type_name() -> &'static str;
    fn ops() -> Vec<u32>;
    fn descriptor_size() -> u32 {
        std::mem::size_of::<Self>() as u32
    }
    fn descriptor_align() -> u32 {
        std::mem::align_of::<Self>() as u32
    }
    /// # Safety
    ///
    /// `ptr` must point to a valid sample instance of `Self` produced by
    /// CycloneDDS for the current topic descriptor. Implementations must return
    /// an owned Rust value that remains valid after any associated DDS loan is
    /// returned.
    unsafe fn clone_out(ptr: *const Self) -> Self {
        std::ptr::read(ptr)
    }
    fn write_to_native<'a>(&'a self, _arena: &'a mut WriteArena) -> DdsResult<*const c_void> {
        Ok(self as *const Self as *const c_void)
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

pub trait DdsEnumType: Sized + Copy + Send + 'static {
    fn max_discriminant() -> u32;
    fn enum_op_flags() -> u32 {
        2u32 << OP_FLAG_SZ_SHIFT
    }
}

pub trait DdsUnionType: Sized + Send + 'static {
    fn discriminant_type() -> DiscriminantType;
    fn case_count() -> u32;
    fn has_default() -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscriminantType {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
}

/// Describes a key field for a keyed topic. `ops_index` is the index into the ops array
/// path that contributes to this key. For flat keys, `ops_path` has one entry.
/// For nested keys, it contains the parent field ADR index followed by the
/// nested key path indices.
pub struct KeyDescriptor {
    pub name: String,
    pub ops_path: Vec<u32>,
}

pub fn adr(typecode: u32, offset: u32) -> Vec<u32> {
    vec![OP_ADR | typecode, offset]
}

pub fn adr_key(typecode: u32, offset: u32) -> Vec<u32> {
    vec![OP_ADR | OP_FLAG_KEY | typecode, offset]
}

pub fn adr_bst(offset: u32, bound: u32) -> Vec<u32> {
    vec![OP_ADR | TYPE_BST, offset, bound]
}

pub fn rebase_ops(mut ops: Vec<u32>, base_offset: u32) -> Vec<u32> {
    let mut i = 0usize;
    while i < ops.len() {
        let op = ops[i];
        if (op & DDS_OP_MASK) != OP_ADR {
            i += 1;
            continue;
        }

        if i + 1 >= ops.len() {
            break;
        }
        ops[i + 1] = ops[i + 1].saturating_add(base_offset);

        let primary = op & DDS_OP_TYPE_MASK;
        let subtype = op & DDS_OP_SUBTYPE_MASK;
        i += match primary {
            TYPE_BST => 3,
            TYPE_SEQ => {
                if subtype == SUBTYPE_BST {
                    3
                } else {
                    2
                }
            }
            TYPE_BSQ => {
                if subtype == SUBTYPE_BST {
                    4
                } else {
                    3
                }
            }
            _ => 2,
        };
    }
    ops
}

impl<T: DdsType> Topic<T> {
    pub fn new(participant: dds_entity_t, name: &str) -> DdsResult<Self> {
        Self::with_qos(participant, name, None)
    }

    pub fn with_qos(participant: dds_entity_t, name: &str, qos: Option<&Qos>) -> DdsResult<Self> {
        unsafe {
            let type_name = CString::new(T::type_name())
                .map_err(|_| DdsError::BadParameter("type name contains null".into()))?;
            let topic_name = CString::new(name)
                .map_err(|_| DdsError::BadParameter("topic name contains null".into()))?;

            // Ops layout (flat struct, matching idlc output):
            //   [ADR..., ADR..., ..., OP_RTS, KOF|1, adr_idx, KOF|1, adr_idx, ...]
            // No OP_DLC for flat types. KOF entries go AFTER OP_RTS.
            let mut ops = T::ops();
            if ops.last().copied() != Some(OP_RTS) {
                ops.push(OP_RTS);
            }

            let key_defs = T::keys();
            let key_names: Vec<CString> = key_defs
                .iter()
                .map(|k| CString::new(k.name.as_str()).unwrap())
                .collect();
            let mut keys: Vec<dds_key_descriptor> = Vec::with_capacity(key_defs.len());
            for (i, kd) in key_defs.iter().enumerate() {
                let offset = ops.len() as u32;
                ops.push(OP_KOF | (kd.ops_path.len() as u32));
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
            let meta = CString::new("").unwrap();

            let descriptor = dds_topic_descriptor {
                m_size: T::descriptor_size(),
                m_align: T::descriptor_align(),
                m_flagset: T::flagset(),
                m_nkeys: T::key_count() as u32,
                m_typename: type_name.as_ptr(),
                m_keys: if keys.is_empty() {
                    std::ptr::null()
                } else {
                    keys.as_ptr()
                },
                m_nops: ops.len() as u32,
                m_ops: ops.as_ptr(),
                m_meta: meta.as_ptr(),
                type_information: std::mem::zeroed(),
                type_mapping: std::mem::zeroed(),
                restrict_data_representation: 0,
            };

            let handle = dds_create_topic(
                participant,
                &descriptor,
                topic_name.as_ptr(),
                qos.map_or(std::ptr::null(), |q| q.as_ptr()),
                std::ptr::null(),
            );
            crate::error::check_entity(handle)?;

            let holder = DescriptorHolder {
                _ops: ops,
                _typename: type_name,
                _key_names: key_names,
                _keys: keys,
                _meta: meta,
            };

            Ok(Topic {
                entity: handle,
                _holder: Rc::new(holder),
                _marker: PhantomData,
            })
        }
    }
}

impl<T> DdsEntity for Topic<T> {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl<T> Drop for Topic<T> {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
