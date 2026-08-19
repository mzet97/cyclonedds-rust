//! Network type discovery via XTypes type lookup.
//!
//! This module provides the ability to discover type information from remote
//! participants by inspecting matched endpoints and resolving type identifiers
//! into full type definitions.

use crate::{
    dynamic_value::{DynamicFieldSchema, DynamicTypeSchema},
    xtypes::{FindScope, MatchedEndpoint, TopicDescriptor, TypeInfo},
    DataReader, DataWriter, DdsEntity, DdsError, DdsResult, DomainParticipant,
};
use cyclonedds_rust_sys::*;
use std::ffi::CStr;

// ---------------------------------------------------------------------------
// DiscoveredType
// ---------------------------------------------------------------------------

/// The result of a type discovery operation.
///
/// Contains the main type's schema and the name of the type.
#[derive(Debug, Clone)]
pub struct DiscoveredType {
    /// The type name (e.g., "MyModule::MyStruct").
    pub type_name: String,
    /// The main type's schema describing its structure.
    pub type_schema: DynamicTypeSchema,
    /// The topic descriptor from which the schema was derived.
    /// Can be used to create topics for reading/writing this type.
    pub topic_descriptor: TopicDescriptor,
}

impl DiscoveredType {
    /// Create a topic for this discovered type on the given participant.
    pub fn create_topic(
        &self,
        participant: &DomainParticipant,
        topic_name: &str,
    ) -> DdsResult<crate::UntypedTopic> {
        self.topic_descriptor.create_topic(participant, topic_name)
    }

    /// Create a topic for this discovered type with QoS.
    pub fn create_topic_with_qos(
        &self,
        participant: &DomainParticipant,
        topic_name: &str,
        qos: &crate::Qos,
    ) -> DdsResult<crate::UntypedTopic> {
        self.topic_descriptor
            .create_topic_with_qos(participant, topic_name, qos)
    }
}

// ---------------------------------------------------------------------------
// Type discovery functions
// ---------------------------------------------------------------------------

/// Discover the type of a matched publication endpoint.
///
/// Obtains the type information from a matched publication (writer on a
/// remote participant) and resolves it into a full type schema that can
/// be used to create a local reader for that type.
///
/// This is the primary mechanism for dynamic subscription: you discover
/// what type a remote writer publishes, then create a local reader using
/// the discovered type information.
pub fn discover_type_from_publication(
    participant: &DomainParticipant,
    reader: &DataReader<impl crate::DdsType>,
    publication_handle: dds_instance_handle_t,
    timeout: dds_duration_t,
) -> DdsResult<DiscoveredType> {
    let endpoint = MatchedEndpoint::from_publication(reader.entity(), publication_handle)?;
    discover_type_from_endpoint(participant, &endpoint, timeout)
}

/// Discover the type of a matched subscription endpoint.
///
/// Obtains the type information from a matched subscription (reader on a
/// remote participant) and resolves it into a full type schema.
pub fn discover_type_from_subscription(
    participant: &DomainParticipant,
    writer: &DataWriter<impl crate::DdsType>,
    subscription_handle: dds_instance_handle_t,
    timeout: dds_duration_t,
) -> DdsResult<DiscoveredType> {
    let endpoint = MatchedEndpoint::from_subscription(writer.entity(), subscription_handle)?;
    discover_type_from_endpoint(participant, &endpoint, timeout)
}

/// Discover a type from any matched endpoint.
///
/// Given a `MatchedEndpoint` (obtained from `matched_publication_endpoints()`
/// or `matched_subscription_endpoints()`), resolve the full type information.
pub fn discover_type_from_endpoint(
    participant: &DomainParticipant,
    endpoint: &MatchedEndpoint,
    timeout: dds_duration_t,
) -> DdsResult<DiscoveredType> {
    let type_info = endpoint.type_info()?;
    let type_name = endpoint.type_name();
    let descriptor = type_info.create_topic_descriptor(participant, FindScope::Global, timeout)?;
    let schema = type_schema_from_descriptor(&descriptor, &type_name)?;

    Ok(DiscoveredType {
        type_name,
        type_schema: schema,
        topic_descriptor: descriptor,
    })
}

/// Discover a type directly from a `TypeInfo` obtained from any entity.
///
/// This resolves the type information into a topic descriptor and schema.
#[cfg_attr(feature = "tracing", tracing::instrument(skip(participant, type_info)))]
pub fn discover_type_from_type_info(
    participant: &DomainParticipant,
    type_info: &TypeInfo,
    type_name: &str,
    timeout: dds_duration_t,
) -> DdsResult<DiscoveredType> {
    let descriptor = type_info.create_topic_descriptor(participant, FindScope::Global, timeout)?;
    let schema = type_schema_from_descriptor(&descriptor, type_name)?;

    Ok(DiscoveredType {
        type_name: type_name.to_string(),
        type_schema: schema,
        topic_descriptor: descriptor,
    })
}

/// Get the type info from a matched publication and resolve it.
///
/// Convenience function that chains getting the matched publications with
/// type discovery. Returns discovered types for all matched publications.
pub fn discover_all_publication_types(
    participant: &DomainParticipant,
    reader: &DataReader<impl crate::DdsType>,
    timeout: dds_duration_t,
) -> DdsResult<Vec<DiscoveredType>> {
    let endpoints = reader.matched_publication_endpoints()?;
    let mut results = Vec::with_capacity(endpoints.len());
    for endpoint in &endpoints {
        match discover_type_from_endpoint(participant, endpoint, timeout) {
            Ok(dt) => results.push(dt),
            Err(_) => continue, // skip endpoints whose type we can't resolve
        }
    }
    Ok(results)
}

/// Get the type info from a matched subscription and resolve it.
///
/// Convenience function that chains getting the matched subscriptions with
/// type discovery. Returns discovered types for all matched subscriptions.
pub fn discover_all_subscription_types(
    participant: &DomainParticipant,
    writer: &DataWriter<impl crate::DdsType>,
    timeout: dds_duration_t,
) -> DdsResult<Vec<DiscoveredType>> {
    let endpoints = writer.matched_subscription_endpoints()?;
    let mut results = Vec::with_capacity(endpoints.len());
    for endpoint in &endpoints {
        match discover_type_from_endpoint(participant, endpoint, timeout) {
            Ok(dt) => results.push(dt),
            Err(_) => continue,
        }
    }
    Ok(results)
}

// ---------------------------------------------------------------------------
// Schema extraction from topic descriptor
// ---------------------------------------------------------------------------

/// Build a `DynamicTypeSchema` from a topic descriptor's ops array.
///
/// The topic descriptor contains the serialization ops array and metadata
/// that fully describe the type. We parse the ops to reconstruct the schema.
fn type_schema_from_descriptor(
    descriptor: &TopicDescriptor,
    type_name: &str,
) -> DdsResult<DynamicTypeSchema> {
    let ops = descriptor.ops();
    let flagset = descriptor.flagset();

    // Determine extensibility from flagset
    let extensibility = if flagset & crate::OP_FLAG_EXT != 0 {
        // Check for mutable vs appendable via metadata XML
        let xml = descriptor.metadata_xml();
        if xml.contains("xcdr2") || xml.contains("appendable") || xml.contains("@appendable") {
            Some(crate::DynamicTypeExtensibility::Appendable)
        } else if xml.contains("mutable") || xml.contains("@mutable") {
            Some(crate::DynamicTypeExtensibility::Mutable)
        } else {
            Some(crate::DynamicTypeExtensibility::Appendable)
        }
    } else {
        Some(crate::DynamicTypeExtensibility::Final)
    };

    // Parse the ops array to extract field information
    let fields = parse_ops_to_fields(ops, descriptor.size())?;

    Ok(DynamicTypeSchema::Struct {
        name: type_name.to_string(),
        base: None,
        fields,
        extensibility,
        autoid: None,
        nested: false,
    })
}

/// Parse a CDR ops array into field schemas.
///
/// The ops array is a sequence of opcodes that describe how to serialize
/// a type. We extract field names from the key descriptors and types from
/// the opcodes.
fn parse_ops_to_fields(ops: &[u32], _struct_size: u32) -> DdsResult<Vec<DynamicFieldSchema>> {
    use crate::dynamic_type::DynamicPrimitiveKind;
    use crate::dynamic_value::DynamicTypeSchema as Sch;
    use crate::topic::*;

    let mut fields = Vec::new();
    let mut i = 0usize;
    let mut field_index = 0u32;

    while i < ops.len() {
        let op = ops[i];
        let opcode = op & DDS_OP_MASK;

        match opcode {
            OP_ADR => {
                let primary = op & DDS_OP_TYPE_MASK;
                let subtype = op & DDS_OP_SUBTYPE_MASK_CONST;

                let field_schema = match primary {
                    TYPE_1BY => {
                        if op & OP_FLAG_SGN != 0 {
                            Sch::Primitive(DynamicPrimitiveKind::Int8)
                        } else {
                            Sch::Primitive(DynamicPrimitiveKind::UInt8)
                        }
                    }
                    TYPE_2BY => {
                        if op & OP_FLAG_SGN != 0 {
                            Sch::Primitive(DynamicPrimitiveKind::Int16)
                        } else {
                            Sch::Primitive(DynamicPrimitiveKind::UInt16)
                        }
                    }
                    TYPE_4BY => {
                        if subtype == SUBTYPE_ENU {
                            Sch::Enum {
                                name: String::new(),
                                literals: Vec::new(),
                                bit_bound: None,
                            }
                        } else if op & OP_FLAG_FP != 0 {
                            Sch::Primitive(DynamicPrimitiveKind::Float32)
                        } else if op & OP_FLAG_SGN != 0 {
                            Sch::Primitive(DynamicPrimitiveKind::Int32)
                        } else {
                            Sch::Primitive(DynamicPrimitiveKind::UInt32)
                        }
                    }
                    TYPE_8BY => {
                        if op & OP_FLAG_FP != 0 {
                            Sch::Primitive(DynamicPrimitiveKind::Float64)
                        } else if op & OP_FLAG_SGN != 0 {
                            Sch::Primitive(DynamicPrimitiveKind::Int64)
                        } else {
                            Sch::Primitive(DynamicPrimitiveKind::UInt64)
                        }
                    }
                    TYPE_STR => Sch::String { bound: None },
                    TYPE_BST => {
                        let capacity = *ops.get(i + 2).ok_or_else(|| {
                            DdsError::BadParameter(
                                "bounded string descriptor is missing its capacity".into(),
                            )
                        })?;
                        let bound = capacity.checked_sub(1).ok_or_else(|| {
                            DdsError::BadParameter(
                                "bounded string descriptor capacity must include a terminator"
                                    .into(),
                            )
                        })?;
                        Sch::String { bound: Some(bound) }
                    }
                    TYPE_SEQ => Sch::Sequence {
                        name: String::new(),
                        bound: None,
                        element: Box::new(Sch::Primitive(DynamicPrimitiveKind::Int8)),
                    },
                    TYPE_ARR => Sch::Array {
                        name: String::new(),
                        bounds: vec![0],
                        element: Box::new(Sch::Primitive(DynamicPrimitiveKind::Int8)),
                    },
                    TYPE_EXT => Sch::Struct {
                        name: String::new(),
                        base: None,
                        fields: Vec::new(),
                        extensibility: None,
                        autoid: None,
                        nested: true,
                    },
                    _ => Sch::Primitive(DynamicPrimitiveKind::Int8),
                };

                let is_key = op & OP_FLAG_KEY != 0;
                let is_optional = op & OP_FLAG_OPT != 0;

                fields.push(DynamicFieldSchema {
                    name: format!("field_{}", field_index),
                    member_id: field_index,
                    value: field_schema,
                    optional: is_optional,
                    key: is_key,
                    external: false,
                    must_understand: false,
                    hash_id_name: None,
                });

                field_index += 1;

                // Advance past this ADR op
                i += crate::xtypes::adr_step(ops, i);
            }
            OP_RTS | OP_DLC | OP_JEQ4 | OP_KOF | OP_MID => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(fields)
}

// ---------------------------------------------------------------------------
// DynamicData CDR I/O helpers
// ---------------------------------------------------------------------------

struct DynamicCdrDescriptor {
    inner: dds_cdrstream_desc,
    _key_names: Vec<std::ffi::CString>,
    _keys: Vec<dds_key_descriptor>,
}

impl DynamicCdrDescriptor {
    fn new(descriptor: &TopicDescriptor) -> DdsResult<Self> {
        let key_defs = descriptor.key_descriptors();
        let key_names: Vec<std::ffi::CString> = key_defs
            .iter()
            .map(|key| {
                std::ffi::CString::new(key.name.as_str())
                    .map_err(|_| DdsError::BadParameter("key name contains null".into()))
            })
            .collect::<DdsResult<_>>()?;
        let keys: Vec<dds_key_descriptor> = key_defs
            .iter()
            .enumerate()
            .map(|(index, key)| dds_key_descriptor {
                m_name: key_names[index].as_ptr(),
                m_offset: key.offset,
                m_idx: key.index,
            })
            .collect();
        let key_count = u32::try_from(keys.len())
            .map_err(|_| DdsError::BadParameter("too many dynamic type keys".into()))?;

        let mut inner = dds_cdrstream_desc::default();
        // SAFETY: [Category 8 — FFI boundary] `inner` is writable initialized
        // storage; the topic-owned ops and locally owned key pointers remain
        // alive for the call, and CycloneDDS copies both into `inner`.
        unsafe {
            dds_cdrstream_desc_init(
                &mut inner,
                &dds_cdrstream_default_allocator,
                descriptor.size(),
                descriptor.align(),
                descriptor.flagset(),
                descriptor.ops().as_ptr(),
                if keys.is_empty() {
                    std::ptr::null()
                } else {
                    keys.as_ptr()
                },
                key_count,
            );
        }
        Ok(Self {
            inner,
            _key_names: key_names,
            _keys: keys,
        })
    }

    fn as_ptr(&self) -> *const dds_cdrstream_desc {
        &self.inner
    }
}

impl Drop for DynamicCdrDescriptor {
    fn drop(&mut self) {
        // SAFETY: [Category 12 — Double Free] `inner` was initialized exactly
        // once by `new` and is finalized only by this unique RAII owner.
        unsafe {
            dds_cdrstream_desc_fini(&mut self.inner, &dds_cdrstream_default_allocator);
        }
    }
}

struct DynamicNativeSample<'a> {
    ptr: std::ptr::NonNull<u8>,
    layout: std::alloc::Layout,
    ops: &'a [u32],
}

impl<'a> DynamicNativeSample<'a> {
    fn new(descriptor: &'a TopicDescriptor) -> DdsResult<Self> {
        let size = usize::try_from(descriptor.size())
            .map_err(|_| DdsError::BadParameter("dynamic type size is too large".into()))?;
        if size == 0 {
            return Err(DdsError::BadParameter(
                "dynamic type size must be non-zero".into(),
            ));
        }
        let align = usize::try_from(descriptor.align().max(1))
            .map_err(|_| DdsError::BadParameter("dynamic type alignment is too large".into()))?;
        let layout = std::alloc::Layout::from_size_align(size, align)
            .map_err(|_| DdsError::BadParameter("invalid type layout for dynamic data".into()))?;
        // SAFETY: [Category 4 — Uninitialized Memory] `layout` is non-zero and
        // valid; zero-initialization makes all descriptor-owned pointer members
        // null so `dds_stream_free_sample` is valid on every later error path.
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        let ptr = std::ptr::NonNull::new(ptr).ok_or(DdsError::OutOfMemory)?;
        Ok(Self {
            ptr,
            layout,
            ops: descriptor.ops(),
        })
    }

    fn as_ptr(&self) -> *const std::ffi::c_void {
        self.ptr.as_ptr().cast()
    }

    fn as_mut_ptr(&mut self) -> *mut std::ffi::c_void {
        self.ptr.as_ptr().cast()
    }

    fn as_mut_bytes(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }
}

impl Drop for DynamicNativeSample<'_> {
    fn drop(&mut self) {
        // SAFETY: [Categories 3 and 12 — Use After Free / Double Free] this
        // unique owner keeps the zeroed or fully initialized sample alive,
        // releases descriptor-owned members first, then deallocates its storage
        // exactly once with the original layout.
        unsafe {
            dds_stream_free_sample(
                self.ptr.as_ptr().cast(),
                &dds_cdrstream_default_allocator,
                self.ops.as_ptr(),
            );
            std::alloc::dealloc(self.ptr.as_ptr(), self.layout);
        }
    }
}

struct DynamicInputStream<'a> {
    inner: dds_istream_t,
    _data: std::marker::PhantomData<&'a [u8]>,
}

impl<'a> DynamicInputStream<'a> {
    fn new(data: &'a [u8]) -> DdsResult<Self> {
        let size = u32::try_from(data.len())
            .map_err(|_| DdsError::BadParameter("CDR input is too large".into()))?;
        let mut inner = dds_istream_t::default();
        // SAFETY: [Category 8 — FFI boundary] `data` remains borrowed by this
        // guard for the stream lifetime, and `size` exactly describes it.
        unsafe {
            dds_istream_init(&mut inner, size, data.as_ptr().cast(), 1);
        }
        Ok(Self {
            inner,
            _data: std::marker::PhantomData,
        })
    }

    fn as_mut_ptr(&mut self) -> *mut dds_istream_t {
        &mut self.inner
    }
}

impl Drop for DynamicInputStream<'_> {
    fn drop(&mut self) {
        // SAFETY: [Category 12 — Double Free] this guard exclusively owns one
        // initialized input stream and finalizes it exactly once.
        unsafe { dds_istream_fini(&mut self.inner) };
    }
}

struct DynamicOutputStream {
    inner: dds_ostream_t,
}

impl DynamicOutputStream {
    fn new() -> Self {
        let mut inner = dds_ostream_t::default();
        // SAFETY: [Category 8 — FFI boundary] `inner` is writable initialized
        // storage and this guard retains exclusive ownership until finalization.
        unsafe { dds_ostream_init(&mut inner, &dds_cdrstream_default_allocator, 0, 1) };
        Self { inner }
    }

    fn as_mut_ptr(&mut self) -> *mut dds_ostream_t {
        &mut self.inner
    }

    fn to_vec(&self) -> DdsResult<Vec<u8>> {
        let len = usize::try_from(self.inner.m_index)
            .map_err(|_| DdsError::BadParameter("serialized CDR size is too large".into()))?;
        if len == 0 {
            return Ok(Vec::new());
        }
        let buffer = std::ptr::NonNull::new(self.inner.m_buffer)
            .ok_or_else(|| DdsError::Other("CDR writer returned a null buffer".into()))?;
        let mut bytes = vec![0u8; len];
        // SAFETY: [Category 10 — Out of Bounds] CycloneDDS sets `m_index` to
        // the initialized byte count within its non-null output allocation;
        // `bytes` was allocated to that exact checked length.
        unsafe { std::ptr::copy_nonoverlapping(buffer.as_ptr(), bytes.as_mut_ptr(), len) };
        Ok(bytes)
    }
}

impl Drop for DynamicOutputStream {
    fn drop(&mut self) {
        // SAFETY: [Category 12 — Double Free] this guard exclusively owns one
        // initialized output stream and finalizes it exactly once.
        unsafe { dds_ostream_fini(&mut self.inner, &dds_cdrstream_default_allocator) };
    }
}

fn normalize_dynamic_cdr(cdr_data: &[u8], descriptor: &DynamicCdrDescriptor) -> DdsResult<Vec<u8>> {
    let size = u32::try_from(cdr_data.len())
        .map_err(|_| DdsError::BadParameter("CDR input is too large".into()))?;
    let mut normalized = cdr_data.to_vec();
    let mut actual_size = 0u32;
    // SAFETY: [Categories 8 and 10 — FFI Boundary / Out of Bounds]
    // `normalized` owns exactly `size` writable bytes and `descriptor` is live;
    // CycloneDDS's normalizer validates every descriptor-directed access.
    let valid = unsafe {
        dds_stream_normalize(
            normalized.as_mut_ptr().cast(),
            size,
            false,
            1,
            descriptor.as_ptr(),
            false,
            &mut actual_size,
        )
    };
    if !valid {
        return Err(DdsError::BadParameter(
            "malformed CDR data for this dynamic type".into(),
        ));
    }
    let actual_size = usize::try_from(actual_size)
        .map_err(|_| DdsError::BadParameter("normalized CDR size is too large".into()))?;
    if actual_size > normalized.len() {
        return Err(DdsError::BadParameter(
            "normalizer returned an invalid CDR size".into(),
        ));
    }
    normalized.truncate(actual_size);
    Ok(normalized)
}

/// Serialize a `DynamicData` value to CDR bytes using the topic descriptor
/// obtained from a dynamic type registration.
///
/// This writes the dynamic value into a native sample buffer matching the
/// topic descriptor's layout, then serializes it to CDR.
///
/// Returns the CDR bytes (including encoding header).
pub fn dynamic_data_to_cdr(
    data: &crate::DynamicData,
    descriptor: &TopicDescriptor,
) -> DdsResult<Vec<u8>> {
    validate_data_for_native(data, descriptor.ops())?;
    let stream_descriptor = DynamicCdrDescriptor::new(descriptor)?;
    let mut native = DynamicNativeSample::new(descriptor)?;
    write_data_to_native(data, native.as_mut_bytes(), descriptor.ops())?;
    let mut output = DynamicOutputStream::new();
    // SAFETY: [Category 8 — FFI boundary] the RAII guards keep the initialized
    // descriptor, native sample, and output stream alive and uniquely owned.
    let wrote = unsafe {
        dds_stream_write_sample(
            output.as_mut_ptr(),
            &dds_cdrstream_default_allocator,
            native.as_ptr(),
            stream_descriptor.as_ptr(),
        )
    };
    if !wrote {
        return Err(DdsError::Unsupported(
            "CDR serialization of dynamic data failed".into(),
        ));
    }
    output.to_vec()
}

/// Deserialize CDR bytes into a `DynamicData` value using the given schema
/// and topic descriptor.
///
/// This reads CDR bytes into a native sample buffer matching the topic
/// descriptor's layout, then extracts the field values into a `DynamicValue`.
pub fn cdr_to_dynamic_data(
    cdr_data: &[u8],
    schema: &crate::DynamicTypeSchema,
    descriptor: &TopicDescriptor,
) -> DdsResult<crate::DynamicData> {
    let stream_descriptor = DynamicCdrDescriptor::new(descriptor)?;
    let normalized = normalize_dynamic_cdr(cdr_data, &stream_descriptor)?;
    let mut input = DynamicInputStream::new(&normalized)?;
    let mut native = DynamicNativeSample::new(descriptor)?;
    // SAFETY: [Category 8 — FFI boundary] normalization proved the borrowed
    // stream is well formed for this live descriptor; the unique zeroed native
    // sample is large and aligned according to the same topic descriptor.
    unsafe {
        dds_stream_read_sample(
            input.as_mut_ptr(),
            native.as_mut_ptr(),
            &dds_cdrstream_default_allocator,
            stream_descriptor.as_ptr(),
        );
    }
    let value = read_value_from_native(native.as_mut_bytes(), schema, descriptor.ops(), 0)?;
    crate::DynamicData::from_value(schema, value)
}

// ---------------------------------------------------------------------------
// Native buffer <-> DynamicValue conversion
// ---------------------------------------------------------------------------

/// Write a `DynamicValue` into a native sample buffer at the given base offset,
/// following the ops array to determine field positions.
pub(crate) fn write_data_to_native(
    data: &crate::DynamicData,
    base: *mut u8,
    ops: &[u32],
) -> DdsResult<()> {
    use crate::dynamic_value::DynamicValue as DV;
    use crate::topic::*;

    validate_data_for_native(data, ops)?;

    let struct_fields = match data.value() {
        DV::Struct(fields) => fields,
        _ => {
            return Err(DdsError::BadParameter(
                "dynamic native value must be a struct".into(),
            ))
        }
    };
    let schema_fields = match data.schema() {
        crate::DynamicTypeSchema::Struct { fields, .. } => fields,
        _ => {
            return Err(DdsError::BadParameter(
                "dynamic native schema must be a struct".into(),
            ))
        }
    };

    let mut i = 0usize;
    let mut field_index = 0usize;
    while i < ops.len() {
        let op = ops[i];
        let opcode = op & DDS_OP_MASK;

        match opcode {
            OP_ADR => {
                let primary = op & DDS_OP_TYPE_MASK;
                let offset = if i + 1 < ops.len() {
                    ops[i + 1] as usize
                } else {
                    0
                };
                // SAFETY: the CycloneDDS descriptor guarantees each field offset
                // lies within the native sample allocation associated with `base`.
                let dst = unsafe { base.add(offset) };

                let field_schema = schema_fields.get(field_index).ok_or_else(|| {
                    DdsError::BadParameter(
                        "native descriptor has more fields than the dynamic schema".into(),
                    )
                })?;
                let field_val = struct_fields.get(&field_schema.name).ok_or_else(|| {
                    DdsError::BadParameter(format!(
                        "missing required dynamic field '{}'",
                        field_schema.name
                    ))
                })?;
                let capacity = if primary == TYPE_BST {
                    Some(bounded_string_capacity(ops, i)?)
                } else {
                    None
                };
                write_primitive_to_native(dst, field_val, primary, capacity)?;
                field_index += 1;

                // Advance
                i += crate::xtypes::adr_step(ops, i);
            }
            OP_RTS | OP_DLC => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    if field_index != schema_fields.len() {
        return Err(DdsError::BadParameter(
            "dynamic schema has more fields than the native descriptor".into(),
        ));
    }

    Ok(())
}

pub(crate) fn validate_data_for_native(data: &crate::DynamicData, ops: &[u32]) -> DdsResult<()> {
    use crate::dynamic_value::DynamicValue as DV;
    use crate::topic::*;

    data.validate()?;
    let fields = match data.value() {
        DV::Struct(fields) => fields,
        _ => {
            return Err(DdsError::BadParameter(
                "dynamic native value must be a struct".into(),
            ))
        }
    };
    let schema_fields = match data.schema() {
        crate::DynamicTypeSchema::Struct { fields, .. } => fields,
        _ => {
            return Err(DdsError::BadParameter(
                "dynamic native schema must be a struct".into(),
            ))
        }
    };
    let mut names = std::collections::BTreeSet::new();
    let mut member_ids = std::collections::BTreeSet::new();
    for field in schema_fields {
        if !names.insert(field.name.as_str()) {
            return Err(DdsError::BadParameter(format!(
                "duplicate dynamic field name '{}'",
                field.name
            )));
        }
        if !member_ids.insert(field.member_id) {
            return Err(DdsError::BadParameter(format!(
                "duplicate dynamic member id {}",
                field.member_id
            )));
        }
    }
    if fields.len() != schema_fields.len() {
        return Err(DdsError::BadParameter(format!(
            "dynamic value has {} fields but schema requires {}",
            fields.len(),
            schema_fields.len()
        )));
    }

    let mut i = 0usize;
    let mut field_index = 0usize;
    while i < ops.len() {
        let op = ops[i];
        match op & DDS_OP_MASK {
            OP_ADR => {
                let primary = op & DDS_OP_TYPE_MASK;
                let field_schema = schema_fields.get(field_index).ok_or_else(|| {
                    DdsError::BadParameter(
                        "native descriptor has more fields than the dynamic schema".into(),
                    )
                })?;
                let value = fields.get(&field_schema.name).ok_or_else(|| {
                    DdsError::BadParameter(format!(
                        "missing required dynamic field '{}'",
                        field_schema.name
                    ))
                })?;
                match (primary, value) {
                    (TYPE_STR, DV::String(string)) => {
                        std::ffi::CString::new(string.as_str()).map_err(|_| {
                            DdsError::BadParameter("string contains an interior null".into())
                        })?;
                    }
                    (TYPE_BST, DV::String(string)) => {
                        let capacity = bounded_string_capacity(ops, i)?;
                        let bound = capacity.checked_sub(1).ok_or_else(|| {
                            DdsError::BadParameter(
                                "bounded string capacity must include a terminator".into(),
                            )
                        })?;
                        if string.len() > bound {
                            return Err(DdsError::BadParameter(format!(
                                "string length {} exceeds bound {}",
                                string.len(),
                                bound
                            )));
                        }
                        if string.as_bytes().contains(&0) {
                            return Err(DdsError::BadParameter(
                                "string contains an interior null".into(),
                            ));
                        }
                    }
                    (TYPE_STR | TYPE_BST, _) => {
                        return Err(DdsError::BadParameter(format!(
                            "dynamic field '{}' is not a string",
                            field_schema.name
                        )))
                    }
                    _ => {}
                }
                field_index += 1;
                i += crate::xtypes::adr_step(ops, i);
            }
            OP_RTS | OP_DLC => i += 1,
            _ => i += 1,
        }
    }
    if field_index != schema_fields.len() {
        return Err(DdsError::BadParameter(
            "dynamic schema has more fields than the native descriptor".into(),
        ));
    }
    Ok(())
}

fn bounded_string_capacity(ops: &[u32], op_index: usize) -> DdsResult<usize> {
    let capacity = *ops.get(op_index + 2).ok_or_else(|| {
        DdsError::BadParameter("bounded string descriptor is missing its capacity".into())
    })?;
    let capacity = usize::try_from(capacity)
        .map_err(|_| DdsError::BadParameter("bounded string capacity is too large".into()))?;
    if capacity == 0 {
        return Err(DdsError::BadParameter(
            "bounded string capacity must include a terminator".into(),
        ));
    }
    Ok(capacity)
}

/// Write a single primitive value into the native buffer.
fn write_primitive_to_native(
    dst: *mut u8,
    val: &crate::dynamic_value::DynamicValue,
    primary_type: u32,
    bounded_capacity: Option<usize>,
) -> DdsResult<()> {
    use crate::dynamic_value::DynamicValue as DV;
    use crate::topic::*;

    match (primary_type, val) {
        (TYPE_1BY, DV::Bool(b)) => {
            let v: u8 = if *b { 1 } else { 0 };
            // SAFETY: `dst` is the descriptor-provided address for a u8 field.
            unsafe { std::ptr::write(dst, v) };
        }
        (TYPE_1BY, DV::Int8(v)) => {
            // SAFETY: `dst` is the descriptor-provided address for an i8 field.
            unsafe { std::ptr::write(dst.cast::<i8>(), *v) };
        }
        (TYPE_1BY, DV::UInt8(v)) => {
            // SAFETY: `dst` is the descriptor-provided address for a u8 field.
            unsafe { std::ptr::write(dst, *v) };
        }
        (TYPE_1BY, DV::Byte(v)) => {
            // SAFETY: `dst` is the descriptor-provided address for a byte field.
            unsafe { std::ptr::write(dst, *v) };
        }
        (TYPE_2BY, DV::Int16(v)) => {
            // SAFETY: the descriptor aligns `dst` for this i16 field.
            unsafe { std::ptr::write(dst.cast::<i16>(), *v) };
        }
        (TYPE_2BY, DV::UInt16(v)) => {
            // SAFETY: the descriptor aligns `dst` for this u16 field.
            unsafe { std::ptr::write(dst.cast::<u16>(), *v) };
        }
        (TYPE_4BY, DV::Int32(v)) => {
            // SAFETY: the descriptor aligns `dst` for this i32 field.
            unsafe { std::ptr::write(dst.cast::<i32>(), *v) };
        }
        (TYPE_4BY, DV::UInt32(v)) => {
            // SAFETY: the descriptor aligns `dst` for this u32 field.
            unsafe { std::ptr::write(dst.cast::<u32>(), *v) };
        }
        (TYPE_4BY, DV::Float32(v)) => {
            // SAFETY: the descriptor aligns `dst` for this f32 field.
            unsafe { std::ptr::write(dst.cast::<f32>(), *v) };
        }
        (TYPE_4BY, DV::Enum { value }) => {
            // SAFETY: the descriptor aligns `dst` for this enum's i32 storage.
            unsafe { std::ptr::write(dst.cast::<i32>(), *value) };
        }
        (TYPE_8BY, DV::Int64(v)) => {
            // SAFETY: the descriptor aligns `dst` for this i64 field.
            unsafe { std::ptr::write(dst.cast::<i64>(), *v) };
        }
        (TYPE_8BY, DV::UInt64(v)) => {
            // SAFETY: the descriptor aligns `dst` for this u64 field.
            unsafe { std::ptr::write(dst.cast::<u64>(), *v) };
        }
        (TYPE_8BY, DV::Float64(v)) => {
            // SAFETY: the descriptor aligns `dst` for this f64 field.
            unsafe { std::ptr::write(dst.cast::<f64>(), *v) };
        }
        (TYPE_STR, DV::String(s)) => {
            // DDS string fields store a char* in the native buffer.
            // We need the pointer to outlive this function, so we leak it.
            // This is acceptable because dds_stream_free_sample will clean up
            // the native sample's heap allocations.
            let leaked = std::ffi::CString::new(s.as_str())
                .map_err(|_| DdsError::BadParameter("string contains an interior null".into()))?;
            // SAFETY: `dst` is aligned pointer storage and CycloneDDS takes
            // ownership of this CString through `dds_stream_free_sample`.
            unsafe { std::ptr::write(dst.cast::<*const i8>(), leaked.into_raw()) };
        }
        (TYPE_BST, DV::String(s)) => {
            let bytes = s.as_bytes();
            let capacity = bounded_capacity.ok_or_else(|| {
                DdsError::BadParameter("bounded string capacity is missing".into())
            })?;
            if bytes.len() >= capacity {
                return Err(DdsError::BadParameter(format!(
                    "string length {} exceeds bound {}",
                    bytes.len(),
                    capacity - 1
                )));
            }
            // SAFETY: validation proved `bytes.len() < capacity`; the
            // descriptor allocates exactly `capacity` bytes at `dst`.
            unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len()) };
            // SAFETY: the same bound proof leaves `bytes.len()` in-capacity.
            unsafe { std::ptr::write(dst.add(bytes.len()), 0) };
        }
        _ => {
            // For types we don't handle, leave the zero-initialized buffer
        }
    }
    Ok(())
}

/// Read a `DynamicValue` from a native sample buffer, using the schema
/// to know which fields exist and the ops to find their offsets.
pub(crate) fn read_value_from_native_public(
    base: *mut u8,
    schema: &crate::DynamicTypeSchema,
    ops: &[u32],
    ops_start: usize,
) -> crate::DynamicValue {
    read_value_from_native(base, schema, ops, ops_start).unwrap_or(crate::DynamicValue::Bool(false))
}

fn read_value_from_native(
    base: *mut u8,
    schema: &crate::DynamicTypeSchema,
    ops: &[u32],
    ops_start: usize,
) -> DdsResult<crate::DynamicValue> {
    use crate::dynamic_value::DynamicValue as DV;
    use crate::topic::*;
    use std::collections::BTreeMap;

    let fields_schema = match schema {
        crate::DynamicTypeSchema::Struct { fields, .. } => fields,
        _ => return Ok(schema.default_value()),
    };

    let mut values = BTreeMap::new();
    let mut field_idx = 0usize;
    let mut i = ops_start;

    while i < ops.len() {
        let op = ops[i];
        let opcode = op & DDS_OP_MASK;

        match opcode {
            OP_ADR => {
                let primary = op & DDS_OP_TYPE_MASK;
                let offset = if i + 1 < ops.len() {
                    ops[i + 1] as usize
                } else {
                    0
                };
                // SAFETY: the CycloneDDS descriptor guarantees each field offset
                // lies within the native sample allocation associated with `base`.
                let src = unsafe { base.add(offset) };

                let field_schema = fields_schema.get(field_idx);
                let capacity = if primary == TYPE_BST {
                    Some(bounded_string_capacity(ops, i)?)
                } else {
                    None
                };
                let val = read_primitive_from_native(src, primary, op, field_schema, capacity)?;

                let name = field_schema
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| format!("field_{}", field_idx));

                values.insert(name, val);
                field_idx += 1;

                i += crate::xtypes::adr_step(ops, i);
            }
            OP_RTS | OP_DLC => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    Ok(DV::Struct(values))
}

fn decode_bounded_string(bytes: &[u8]) -> DdsResult<String> {
    let terminator = bytes.iter().position(|byte| *byte == 0).ok_or_else(|| {
        DdsError::BadParameter("bounded native string is not null-terminated".into())
    })?;
    Ok(String::from_utf8_lossy(&bytes[..terminator]).into_owned())
}

/// Read a single primitive value from the native buffer.
fn read_primitive_from_native(
    src: *mut u8,
    primary_type: u32,
    op: u32,
    field_schema: Option<&crate::dynamic_value::DynamicFieldSchema>,
    bounded_capacity: Option<usize>,
) -> DdsResult<crate::dynamic_value::DynamicValue> {
    use crate::dynamic_value::DynamicValue as DV;
    use crate::topic::*;

    let value = unsafe {
        match primary_type {
            TYPE_1BY => {
                let v = std::ptr::read(src as *const u8);
                if let Some(fs) = field_schema {
                    match &fs.value {
                        crate::DynamicTypeSchema::Primitive(
                            crate::dynamic_type::DynamicPrimitiveKind::Boolean,
                        ) => DV::Bool(v != 0),
                        crate::DynamicTypeSchema::Primitive(
                            crate::dynamic_type::DynamicPrimitiveKind::Int8,
                        ) => DV::Int8(v as i8),
                        _ => DV::Byte(v),
                    }
                } else if op & OP_FLAG_SGN != 0 {
                    DV::Int8(v as i8)
                } else {
                    DV::Byte(v)
                }
            }
            TYPE_2BY => {
                let v = std::ptr::read(src as *const i16);
                let vu = std::ptr::read(src as *const u16);
                if op & OP_FLAG_SGN != 0 {
                    DV::Int16(v)
                } else {
                    DV::UInt16(vu)
                }
            }
            TYPE_4BY => {
                if op & OP_FLAG_FP != 0 {
                    DV::Float32(std::ptr::read(src as *const f32))
                } else if let Some(fs) = field_schema {
                    match &fs.value {
                        crate::DynamicTypeSchema::Enum { .. } => DV::Enum {
                            value: std::ptr::read(src as *const i32),
                        },
                        _ => {
                            let v = std::ptr::read(src as *const i32);
                            let vu = std::ptr::read(src as *const u32);
                            if op & OP_FLAG_SGN != 0 {
                                DV::Int32(v)
                            } else {
                                DV::UInt32(vu)
                            }
                        }
                    }
                } else {
                    let v = std::ptr::read(src as *const i32);
                    let vu = std::ptr::read(src as *const u32);
                    if op & OP_FLAG_SGN != 0 {
                        DV::Int32(v)
                    } else {
                        DV::UInt32(vu)
                    }
                }
            }
            TYPE_8BY => {
                if op & OP_FLAG_FP != 0 {
                    DV::Float64(std::ptr::read(src as *const f64))
                } else {
                    let v = std::ptr::read(src as *const i64);
                    let vu = std::ptr::read(src as *const u64);
                    if op & OP_FLAG_SGN != 0 {
                        DV::Int64(v)
                    } else {
                        DV::UInt64(vu)
                    }
                }
            }
            TYPE_STR => {
                let ptr = std::ptr::read(src as *const *const i8);
                if ptr.is_null() {
                    DV::String(String::new())
                } else {
                    DV::String(CStr::from_ptr(ptr).to_string_lossy().into_owned())
                }
            }
            TYPE_BST => {
                let capacity = bounded_capacity.ok_or_else(|| {
                    DdsError::BadParameter("bounded string capacity is missing".into())
                })?;
                // SAFETY: the descriptor allocates `capacity` bytes at `src`;
                // the slice is scanned only within that allocation.
                let bytes = std::slice::from_raw_parts(src.cast_const(), capacity);
                DV::String(decode_bounded_string(bytes)?)
            }
            _ => DV::Null,
        }
    };
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_type::DynamicPrimitiveKind;
    use crate::dynamic_value::{DynamicTypeSchema, DynamicValue};

    fn dynamic_field(name: &str, member_id: u32, kind: DynamicPrimitiveKind) -> DynamicFieldSchema {
        DynamicFieldSchema {
            name: name.to_string(),
            member_id,
            value: DynamicTypeSchema::Primitive(kind),
            optional: false,
            key: false,
            external: false,
            must_understand: false,
            hash_id_name: None,
        }
    }

    fn bounded_string_schema(bound: u32) -> DynamicTypeSchema {
        DynamicTypeSchema::Struct {
            name: "BoundedString".to_string(),
            base: None,
            fields: vec![DynamicFieldSchema {
                name: "field_0".to_string(),
                member_id: 0,
                value: DynamicTypeSchema::String { bound: Some(bound) },
                optional: false,
                key: false,
                external: false,
                must_understand: false,
                hash_id_name: None,
            }],
            extensibility: None,
            autoid: None,
            nested: false,
        }
    }

    #[test]
    fn public_dynamic_data_accepts_string_at_declared_bound() {
        // Given: the public DynamicData API with an IDL string<4> schema.
        let mut data = crate::DynamicData::new(&bounded_string_schema(4));

        // When: a four-byte value is assigned.
        let result = data.set_string("field_0", "four");

        // Then: the exact-bound value is accepted unchanged.
        assert!(result.is_ok());
        assert_eq!(data.get_string("field_0").unwrap(), "four");
    }

    #[test]
    fn descriptor_capacity_maps_to_idl_string_bound() {
        // Given: CycloneDDS TYPE_BST encodes string<4> as five native bytes.
        let ops = [crate::topic::OP_ADR | crate::topic::TYPE_BST, 0, 5];

        // When: the descriptor op stream is converted to a public schema.
        let fields = parse_ops_to_fields(&ops, 5).unwrap();

        // Then: the public bound excludes the required NUL byte.
        assert_eq!(
            fields[0].value,
            DynamicTypeSchema::String { bound: Some(4) }
        );
    }

    #[test]
    fn descriptor_bound_rejects_five_byte_public_value() {
        // Given: a schema derived from a TYPE_BST capacity of five bytes.
        let ops = [crate::topic::OP_ADR | crate::topic::TYPE_BST, 0, 5];
        let fields = parse_ops_to_fields(&ops, 5).unwrap();
        let schema = DynamicTypeSchema::Struct {
            name: "BoundedString".to_string(),
            base: None,
            fields,
            extensibility: None,
            autoid: None,
            nested: false,
        };
        let mut data = crate::DynamicData::new(&schema);

        // When: a five-byte value is assigned through the public setter.
        let result = data.set_string("field_0", "12345");

        // Then: validation returns the typed BadParameter error.
        assert!(matches!(result, Err(DdsError::BadParameter(_))));
    }

    #[test]
    fn zero_capacity_bounded_string_descriptor_is_rejected() {
        // Given: a malformed TYPE_BST descriptor with no room for a NUL.
        let ops = [crate::topic::OP_ADR | crate::topic::TYPE_BST, 0, 0];

        // When: the op stream is parsed.
        let result = parse_ops_to_fields(&ops, 0);

        // Then: checked bound conversion rejects it instead of underflowing.
        assert!(matches!(result, Err(DdsError::BadParameter(_))));
    }

    #[test]
    fn bounded_native_read_rejects_missing_terminator() {
        // Given: all four bytes in a TYPE_BST native capacity are non-NUL.
        let native = [b'a', b'b', b'c', b'd'];

        // When: the bounded native reader scans only that capacity.
        let result = decode_bounded_string(&native);

        // Then: it rejects the malformed field without reading past the array.
        assert!(matches!(result, Err(DdsError::BadParameter(_))));
    }

    #[test]
    fn malformed_bounded_native_value_is_not_replaced_with_valid_default() {
        // Given: a non-terminated native string and its string<3> public schema.
        let mut native = [b'a', b'b', b'c', b'd'];
        let ops = [crate::topic::OP_ADR | crate::topic::TYPE_BST, 0, 4];
        let schema = bounded_string_schema(3);

        // When: the subscription-facing native reader encounters malformed data.
        let value = read_value_from_native_public(native.as_mut_ptr(), &schema, &ops, 0);

        // Then: the value is invalid and cannot pass DynamicData construction.
        assert_eq!(value, DynamicValue::Bool(false));
        assert!(crate::DynamicData::from_value(&schema, value).is_err());
    }

    #[test]
    fn bounded_native_write_accepts_exact_bound_without_overflow() {
        // Given: a string<4> descriptor and guard byte after its capacity.
        let ops = [crate::topic::OP_ADR | crate::topic::TYPE_BST, 0, 5];
        let mut native = [0xa5; 6];
        let schema = bounded_string_schema(4);
        let data = crate::DynamicData::from_value(
            &schema,
            DynamicValue::Struct(std::collections::BTreeMap::from([(
                "field_0".to_string(),
                DynamicValue::String("four".to_string()),
            )])),
        )
        .unwrap();

        // When: the exact-bound value is written to native storage.
        let result = write_data_to_native(&data, native.as_mut_ptr(), &ops);

        // Then: the in-capacity NUL is written and the guard remains untouched.
        assert!(result.is_ok());
        assert_eq!(&native[..5], b"four\0");
        assert_eq!(native[5], 0xa5);
    }

    #[test]
    fn bounded_native_write_rejects_overlength_before_writing() {
        // Given: a string<4> descriptor, overlength value, and sentinel storage.
        let ops = [crate::topic::OP_ADR | crate::topic::TYPE_BST, 0, 5];
        let mut native = [0xa5; 6];
        let schema = bounded_string_schema(5);
        let data = crate::DynamicData::from_value(
            &schema,
            DynamicValue::Struct(std::collections::BTreeMap::from([(
                "field_0".to_string(),
                DynamicValue::String("12345".to_string()),
            )])),
        )
        .unwrap();

        // When: native conversion validates the descriptor capacity.
        let result = write_data_to_native(&data, native.as_mut_ptr(), &ops);

        // Then: it returns a typed error before changing any native byte.
        assert!(matches!(result, Err(DdsError::BadParameter(_))));
        assert_eq!(native, [0xa5; 6]);
    }

    #[test]
    fn native_mapping_rejects_duplicate_schema_names() {
        // Given: two schema members that cannot be represented distinctly by name.
        let schema = DynamicTypeSchema::Struct {
            name: "DuplicateNames".to_string(),
            base: None,
            fields: vec![
                dynamic_field("same", 3, DynamicPrimitiveKind::Int32),
                dynamic_field("same", 9, DynamicPrimitiveKind::Int32),
            ],
            extensibility: Some(crate::DynamicTypeExtensibility::Final),
            autoid: None,
            nested: false,
        };
        let data = crate::DynamicData::new(&schema);
        let ops = [
            crate::topic::OP_ADR | crate::topic::TYPE_4BY,
            0,
            crate::topic::OP_ADR | crate::topic::TYPE_4BY,
            4,
        ];

        // When: the schema crosses the native mapping boundary.
        let result = validate_data_for_native(&data, &ops);

        // Then: ambiguous member identity is a typed input error.
        assert!(matches!(result, Err(DdsError::BadParameter(_))));
    }

    #[test]
    fn native_mapping_rejects_duplicate_member_ids() {
        // Given: distinct names assigned the same DDS member identity.
        let schema = DynamicTypeSchema::Struct {
            name: "DuplicateIds".to_string(),
            base: None,
            fields: vec![
                dynamic_field("left", 7, DynamicPrimitiveKind::Int32),
                dynamic_field("right", 7, DynamicPrimitiveKind::Int32),
            ],
            extensibility: Some(crate::DynamicTypeExtensibility::Final),
            autoid: None,
            nested: false,
        };
        let data = crate::DynamicData::new(&schema);
        let ops = [
            crate::topic::OP_ADR | crate::topic::TYPE_4BY,
            0,
            crate::topic::OP_ADR | crate::topic::TYPE_4BY,
            4,
        ];

        // When: the schema crosses the native mapping boundary.
        let result = validate_data_for_native(&data, &ops);

        // Then: duplicate DDS identity is a typed input error.
        assert!(matches!(result, Err(DdsError::BadParameter(_))));
    }

    #[test]
    fn native_mapping_uses_schema_names_and_declaration_order() {
        // Given: custom names and non-sequential IDs with distinct native values.
        let schema = DynamicTypeSchema::Struct {
            name: "CustomNames".to_string(),
            base: None,
            fields: vec![
                dynamic_field("telemetry_code", 31, DynamicPrimitiveKind::Int32),
                dynamic_field("voltage", 7, DynamicPrimitiveKind::Float64),
            ],
            extensibility: Some(crate::DynamicTypeExtensibility::Final),
            autoid: None,
            nested: false,
        };
        let mut data = crate::DynamicData::new(&schema);
        data.set_i32("telemetry_code", 41_337).unwrap();
        data.set_f64("voltage", 12.75).unwrap();
        let ops = [
            crate::topic::OP_ADR | crate::topic::TYPE_4BY | crate::topic::OP_FLAG_SGN,
            0,
            crate::topic::OP_ADR | crate::topic::TYPE_8BY | crate::topic::OP_FLAG_FP,
            8,
        ];
        let mut native = [0u64; 2];

        // When: schema-directed traversal writes the native layout.
        write_data_to_native(&data, native.as_mut_ptr().cast(), &ops).unwrap();

        // Then: each declaration-order slot contains its named value.
        assert_eq!(
            i32::from_ne_bytes(native[0].to_ne_bytes()[..4].try_into().unwrap()),
            41_337
        );
        assert_eq!(f64::from_bits(native[1]), 12.75);
    }

    #[test]
    fn dynamic_data_get_set_primitives() {
        let schema = DynamicTypeSchema::Struct {
            name: "TestStruct".to_string(),
            base: None,
            fields: vec![
                DynamicFieldSchema {
                    name: "x".to_string(),
                    member_id: 0,
                    value: DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                    optional: false,
                    key: false,
                    external: false,
                    must_understand: false,
                    hash_id_name: None,
                },
                DynamicFieldSchema {
                    name: "y".to_string(),
                    member_id: 1,
                    value: DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Float64),
                    optional: false,
                    key: false,
                    external: false,
                    must_understand: false,
                    hash_id_name: None,
                },
                DynamicFieldSchema {
                    name: "label".to_string(),
                    member_id: 2,
                    value: DynamicTypeSchema::String { bound: None },
                    optional: false,
                    key: false,
                    external: false,
                    must_understand: false,
                    hash_id_name: None,
                },
                DynamicFieldSchema {
                    name: "flag".to_string(),
                    member_id: 3,
                    value: DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Boolean),
                    optional: true,
                    key: false,
                    external: false,
                    must_understand: false,
                    hash_id_name: None,
                },
            ],
            extensibility: None,
            autoid: None,
            nested: false,
        };

        let mut data = crate::DynamicData::new(&schema);

        // Test defaults
        assert_eq!(data.get_i32("x").unwrap(), 0);
        assert_eq!(data.get_f64("y").unwrap(), 0.0);
        assert_eq!(data.get_string("label").unwrap(), "");

        // Test set and get
        data.set_i32("x", 42).unwrap();
        assert_eq!(data.get_i32("x").unwrap(), 42);

        data.set_f64("y", std::f64::consts::PI).unwrap();
        assert!(data.get_f64("y").unwrap().abs() - std::f64::consts::PI < 0.001);

        data.set_string("label", "hello").unwrap();
        assert_eq!(data.get_string("label").unwrap(), "hello");

        data.set_bool("flag", true).unwrap();
        assert!(data.get_bool("flag").unwrap());

        // Test validation passes
        assert!(data.validate().is_ok());

        // Test field names
        let names = data.field_names();
        assert_eq!(names, vec!["x", "y", "label", "flag"]);

        // Test null optional
        data.set_null("flag").unwrap();
        assert!(!data.is_set("flag"));

        // Test non-optional cannot be null
        assert!(data.set_null("x").is_err());

        // Test wrong type access
        assert!(data.get_bool("x").is_err());
        assert!(data.get_i32("label").is_err());
    }

    #[test]
    fn dynamic_data_from_value_validates() {
        let schema = DynamicTypeSchema::Struct {
            name: "S".to_string(),
            base: None,
            fields: vec![DynamicFieldSchema {
                name: "v".to_string(),
                member_id: 0,
                value: DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                optional: false,
                key: false,
                external: false,
                must_understand: false,
                hash_id_name: None,
            }],
            extensibility: None,
            autoid: None,
            nested: false,
        };

        // Valid value
        let val = DynamicValue::Struct({
            let mut m = std::collections::BTreeMap::new();
            m.insert("v".to_string(), DynamicValue::Int32(99));
            m
        });
        let data = crate::DynamicData::from_value(&schema, val).unwrap();
        assert_eq!(data.get_i32("v").unwrap(), 99);

        // Invalid value (wrong type) should fail validation
        let bad_val = DynamicValue::Struct({
            let mut m = std::collections::BTreeMap::new();
            m.insert("v".to_string(), DynamicValue::String("not an i32".into()));
            m
        });
        assert!(crate::DynamicData::from_value(&schema, bad_val).is_err());
    }
}
