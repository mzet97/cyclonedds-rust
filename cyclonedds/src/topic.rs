use crate::{
    entity::{DdsEntity, OwnedEntity, OwnedHandle},
    write_arena::WriteArena,
    xtypes::TopicDescriptor,
    DdsError, DdsResult, Qos,
};
use cyclonedds_rust_sys::*;
use std::ffi::c_void;
use std::ffi::CString;
use std::marker::PhantomData;
use std::sync::Arc;

/// A topic without compile-time type information.
pub struct UntypedTopic {
    inner: Arc<OwnedEntity>,
}

impl UntypedTopic {
    pub(crate) fn from_entity(entity: dds_entity_t) -> Self {
        Self {
            inner: OwnedEntity::unowned(entity, "UntypedTopic"),
        }
    }

    pub(crate) fn adopt(entity: dds_entity_t, parents: Vec<Arc<OwnedEntity>>) -> Self {
        Self {
            inner: OwnedEntity::new(entity, "UntypedTopic", None, parents),
        }
    }

    /// Attach a parent to a topic that was just built without one.
    ///
    /// The `Arc` has to still be unique, which it is for every crate-internal
    /// caller: they call this on the value returned by a constructor, before it
    /// reaches anyone else. If that ever stopped holding, the topic would go
    /// back to not owning its participant — no worse than before this change,
    /// but wrong, so `debug_assert` makes it loud in tests rather than silent.
    pub(crate) fn retaining(mut self, parent: Arc<OwnedEntity>) -> Self {
        match Arc::get_mut(&mut self.inner) {
            Some(inner) => inner.push_parent(parent),
            None => debug_assert!(false, "retaining() on a shared UntypedTopic"),
        }
        self
    }

    /// # Unchecked
    ///
    /// Takes a raw participant handle and does not hold it alive. Prefer
    /// [`DomainParticipant::create_topic_from_descriptor`], which does.
    ///
    /// [`DomainParticipant::create_topic_from_descriptor`]: crate::DomainParticipant::create_topic_from_descriptor
    pub fn from_descriptor(
        participant: dds_entity_t,
        name: &str,
        descriptor: &TopicDescriptor,
    ) -> DdsResult<Self> {
        Self::from_descriptor_with_qos(participant, name, descriptor, None)
    }

    /// See [`UntypedTopic::from_descriptor`].
    pub fn from_descriptor_with_qos(
        participant: dds_entity_t,
        name: &str,
        descriptor: &TopicDescriptor,
        qos: Option<&Qos>,
    ) -> DdsResult<Self> {
        Self::create(participant, name, descriptor, qos, Vec::new())
    }

    pub(crate) fn create(
        participant: dds_entity_t,
        name: &str,
        descriptor: &TopicDescriptor,
        qos: Option<&Qos>,
        parents: Vec<Arc<OwnedEntity>>,
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
            Ok(Self {
                inner: OwnedEntity::new(handle, "UntypedTopic", None, parents),
            })
        }
    }
}

impl DdsEntity for UntypedTopic {
    fn entity(&self) -> dds_entity_t {
        self.inner.handle()
    }
}

impl OwnedHandle for UntypedTopic {
    fn owned(&self) -> &Arc<OwnedEntity> {
        &self.inner
    }
}

pub struct Topic<T> {
    inner: Arc<OwnedEntity>,
    _holder: Arc<DescriptorHolder>,
    _marker: PhantomData<T>,
}

struct DescriptorHolder {
    _ops: Vec<u32>,
    _typename: CString,
    _key_names: Vec<CString>,
    _keys: Vec<dds_key_descriptor>,
    _meta: CString,
}

// Segurança: o holder é imutável após a criação do tópico. Os ponteiros crus em
// `_keys` apontam para os buffers das `CString` em `_key_names` (mesma heap, sem
// realocação depois de publicado), e o CycloneDDS copia o que precisa em
// `dds_create_topic`. Compartilhar entre threads é seguro (somente leitura).
unsafe impl Send for DescriptorHolder {}
unsafe impl Sync for DescriptorHolder {}

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
    /// The DDS wire-compatible representation of this type — what CycloneDDS's
    /// topic descriptor (`m_size`/`m_ops`, see [`DdsType::descriptor_size`]) and
    /// the loan APIs ([`crate::DataWriter::request_loan`]) actually operate on.
    ///
    /// For types with no heap-allocated fields this is typically `Self`. For
    /// types with `String`/`Vec` fields (translated by `#[derive(DdsTypeDerive)]`
    /// to `DdsString`/`DdsSequence`), `Native` is a distinct, smaller,
    /// wire-compatible struct — `size_of::<Native>()` is what
    /// `descriptor_size()` reports and what CycloneDDS actually allocates for a
    /// loan, which is *not* the same as `size_of::<Self>()` in that case.
    /// Getting this wrong was the root cause of a real out-of-bounds write in
    /// `request_loan()` (fixed together with this associated type — see
    /// `tese/src/rust`'s `OPTIMIZATION_PLAN.md` Fase 4 for the full writeup).
    type Native: Sized;

    fn type_name() -> &'static str;
    fn ops() -> Vec<u32>;
    /// Size CycloneDDS allocates for one sample of this type.
    ///
    /// This is the size of [`DdsType::Native`], not of `Self` — they differ for
    /// any type with `String`/`Vec` fields, where the wire representation uses
    /// `DdsString` (8 bytes) / `DdsSequence`. The default used to be
    /// `size_of::<Self>()`, which is only correct when `Native = Self`: a manual
    /// `impl` that declared a smaller `Native` and relied on the default
    /// reintroduced the heap overflow fixed in 2.0.0, since `dds_request_loan`
    /// allocates this many bytes and `request_loan` then zero-initializes
    /// `size_of::<Native>()` of them.
    fn descriptor_size() -> u32 {
        std::mem::size_of::<Self::Native>() as u32
    }
    /// Alignment CycloneDDS uses for one sample. See [`DdsType::descriptor_size`].
    fn descriptor_align() -> u32 {
        std::mem::align_of::<Self::Native>() as u32
    }
    /// # Safety
    ///
    /// `ptr` must point to a valid sample instance of `Self` produced by
    /// CycloneDDS for the current topic descriptor. Implementations must return
    /// an owned Rust value that remains valid after any associated DDS loan is
    /// returned.
    ///
    /// Returns `Err` when the sample cannot be represented as `Self` — the case
    /// that matters is a union carrying a discriminator outside its declared
    /// set, which arrives from the network and so is remote input. That used to
    /// be a `panic!`, contained at the FFI boundary by `catch_unwind` but still
    /// enough for a peer built from a different IDL revision to make
    /// `reader.take()` unwind on the caller's thread. An undecodable sample is
    /// now a discarded error instead.
    unsafe fn clone_out(ptr: *const Self) -> DdsResult<Self> {
        Ok(std::ptr::read(ptr))
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
    /// XCDR2-serialized (TypeInformation, TypeMapping) blobs for XTypes/SEDP.
    ///
    /// Types generated from the canonical IDL carry the blobs produced by the
    /// C `idlc`; returning `Some` makes the topic descriptor set
    /// `DDS_TOPIC_XTYPES_METADATA` and announce type information, which
    /// type-enforcing peers (Python/C++) require for matching.
    fn type_metadata_blobs() -> Option<(&'static [u8], &'static [u8])> {
        None
    }
}

/// Value-level conversion from a Rust sample to its [`DdsType::Native`] layout.
///
/// [`DdsType::write_to_native`] produces a *pointer* into a [`WriteArena`], which
/// is all a top-level write needs. Building the element buffer of a
/// `sequence<Struct>` needs the value itself, because the elements have to sit
/// contiguously with the stride CycloneDDS was told about — and that stride is
/// `size_of::<Native>()`, not `size_of::<Self>()`, for any element type with
/// `String`/`Vec` fields.
///
/// This is a separate trait, and not another method on [`DdsType`], so that the
/// manual `impl DdsType` blocks that already exist for POD types keep compiling.
/// The derive implements it for every type it generates; a hand-written type
/// used as a composite element without it fails with a missing-bound error at
/// compile time rather than mis-serializing at run time.
pub trait DdsNativeValue: DdsType {
    fn to_native_value(&self, arena: &mut WriteArena) -> DdsResult<Self::Native>;
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
            TYPE_SEQ if subtype == SUBTYPE_BST => 3,
            TYPE_SEQ => 2,
            TYPE_BSQ if subtype == SUBTYPE_BST => 4,
            TYPE_BSQ => 3,
            _ => 2,
        };
    }
    ops
}

impl<T: DdsType> Topic<T> {
    /// Create a topic on `participant`.
    ///
    /// Takes the participant by reference rather than as a raw
    /// `dds_entity_t`. The handle-based form let a temporary supply it --
    /// `Topic::new(DomainParticipant::new(0)?.entity(), "x")` compiles, deletes
    /// the participant at the end of the statement, and leaves the topic on a
    /// handle CycloneDDS is free to recycle. See [`Topic::from_entity`] for the
    /// escape hatch when the handle genuinely comes from elsewhere.
    pub fn new(participant: &crate::DomainParticipant, name: &str) -> DdsResult<Self> {
        Self::with_qos(participant, name, None)
    }

    pub fn with_qos(
        participant: &crate::DomainParticipant,
        name: &str,
        qos: Option<&Qos>,
    ) -> DdsResult<Self> {
        Self::create(
            participant.entity(),
            name,
            qos,
            vec![participant.owned().clone()],
        )
    }

    /// Create a topic from a raw participant handle.
    ///
    /// # Unchecked
    ///
    /// Escape hatch for handles obtained outside this crate (FFI interop).
    /// Unlike [`Topic::new`], the returned topic does **not** hold the
    /// participant alive: the caller guarantees the handle is a live
    /// participant and outlives the topic.
    pub fn from_entity(participant: dds_entity_t, name: &str) -> DdsResult<Self> {
        Self::with_qos_from_entity(participant, name, None)
    }

    /// See [`Topic::from_entity`].
    pub fn with_qos_from_entity(
        participant: dds_entity_t,
        name: &str,
        qos: Option<&Qos>,
    ) -> DdsResult<Self> {
        Self::create(participant, name, qos, Vec::new())
    }

    pub(crate) fn create(
        participant: dds_entity_t,
        name: &str,
        qos: Option<&Qos>,
        parents: Vec<Arc<OwnedEntity>>,
    ) -> DdsResult<Self> {
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
            // A key name with an interior NUL is bad input, not a bug: report
            // it like the topic name a few lines above rather than panicking.
            let key_names: Vec<CString> = key_defs
                .iter()
                .map(|k| {
                    CString::new(k.name.as_str()).map_err(|_| {
                        DdsError::BadParameter("key name contains null".into())
                    })
                })
                .collect::<DdsResult<_>>()?;
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
            let meta = CString::default();

            // Blobs XTypes (TypeInformation/TypeMapping do idlc) quando o tipo os tem:
            // habilita DDS_TOPIC_XTYPES_METADATA para o SEDP anunciar type info.
            let (flagset, type_information, type_mapping) = match T::type_metadata_blobs() {
                Some((info, map)) => (
                    T::flagset() | DDS_TOPIC_XTYPES_METADATA,
                    dds_type_meta_ser {
                        data: info.as_ptr(),
                        sz: info.len() as u32,
                    },
                    dds_type_meta_ser {
                        data: map.as_ptr(),
                        sz: map.len() as u32,
                    },
                ),
                None => (T::flagset(), std::mem::zeroed(), std::mem::zeroed()),
            };

            let descriptor = dds_topic_descriptor {
                m_size: T::descriptor_size(),
                m_align: T::descriptor_align(),
                m_flagset: flagset,
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
                type_information,
                type_mapping,
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
                inner: OwnedEntity::new(handle, "Topic", None, parents),
                _holder: Arc::new(holder),
                _marker: PhantomData,
            })
        }
    }
}

impl<T> DdsEntity for Topic<T> {
    fn entity(&self) -> dds_entity_t {
        self.inner.handle()
    }
}

impl<T> OwnedHandle for Topic<T> {
    fn owned(&self) -> &Arc<OwnedEntity> {
        &self.inner
    }
}
