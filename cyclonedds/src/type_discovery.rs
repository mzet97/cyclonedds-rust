//! Network type discovery via XTypes type lookup.
//!
//! This module provides the ability to discover type information from remote
//! participants by inspecting matched endpoints and resolving type identifiers
//! into full type definitions.

use crate::{
    dynamic_value::{DynamicFieldSchema, DynamicTypeSchema},
    xtypes::{FindScope, MatchedEndpoint, TopicDescriptor, TypeInfo},
    DdsEntity, DdsError, DdsResult, DataReader, DataWriter,
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
        participant_entity: dds_entity_t,
        topic_name: &str,
    ) -> DdsResult<crate::UntypedTopic> {
        self.topic_descriptor.create_topic(participant_entity, topic_name)
    }

    /// Create a topic for this discovered type with QoS.
    pub fn create_topic_with_qos(
        &self,
        participant_entity: dds_entity_t,
        topic_name: &str,
        qos: &crate::Qos,
    ) -> DdsResult<crate::UntypedTopic> {
        self.topic_descriptor
            .create_topic_with_qos(participant_entity, topic_name, qos)
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
    reader: &DataReader<impl crate::DdsType>,
    publication_handle: dds_instance_handle_t,
    timeout: dds_duration_t,
) -> DdsResult<DiscoveredType> {
    let endpoint = MatchedEndpoint::from_publication(reader.entity(), publication_handle)?;
    discover_type_from_endpoint(reader.entity(), &endpoint, timeout)
}

/// Discover the type of a matched subscription endpoint.
///
/// Obtains the type information from a matched subscription (reader on a
/// remote participant) and resolves it into a full type schema.
pub fn discover_type_from_subscription(
    writer: &DataWriter<impl crate::DdsType>,
    subscription_handle: dds_instance_handle_t,
    timeout: dds_duration_t,
) -> DdsResult<DiscoveredType> {
    let endpoint = MatchedEndpoint::from_subscription(writer.entity(), subscription_handle)?;
    discover_type_from_endpoint(writer.entity(), &endpoint, timeout)
}

/// Discover a type from any matched endpoint.
///
/// Given a `MatchedEndpoint` (obtained from `matched_publication_endpoints()`
/// or `matched_subscription_endpoints()`), resolve the full type information.
pub fn discover_type_from_endpoint(
    participant_entity: dds_entity_t,
    endpoint: &MatchedEndpoint,
    timeout: dds_duration_t,
) -> DdsResult<DiscoveredType> {
    let type_info = endpoint.type_info()?;
    let type_name = endpoint.type_name();
    let descriptor = type_info.create_topic_descriptor(
        participant_entity,
        FindScope::Global,
        timeout,
    )?;
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
pub fn discover_type_from_type_info(
    participant_entity: dds_entity_t,
    type_info: &TypeInfo,
    type_name: &str,
    timeout: dds_duration_t,
) -> DdsResult<DiscoveredType> {
    let descriptor =
        type_info.create_topic_descriptor(participant_entity, FindScope::Global, timeout)?;
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
    reader: &DataReader<impl crate::DdsType>,
    timeout: dds_duration_t,
) -> DdsResult<Vec<DiscoveredType>> {
    let endpoints = reader.matched_publication_endpoints()?;
    let mut results = Vec::with_capacity(endpoints.len());
    for endpoint in &endpoints {
        match discover_type_from_endpoint(reader.entity(), endpoint, timeout) {
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
    writer: &DataWriter<impl crate::DdsType>,
    timeout: dds_duration_t,
) -> DdsResult<Vec<DiscoveredType>> {
    let endpoints = writer.matched_subscription_endpoints()?;
    let mut results = Vec::with_capacity(endpoints.len());
    for endpoint in &endpoints {
        match discover_type_from_endpoint(writer.entity(), endpoint, timeout) {
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
    let fields = parse_ops_to_fields(ops, descriptor.size());

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
fn parse_ops_to_fields(ops: &[u32], _struct_size: u32) -> Vec<DynamicFieldSchema> {
    use crate::topic::*;
    use crate::dynamic_value::DynamicTypeSchema as Sch;
    use crate::dynamic_type::DynamicPrimitiveKind;

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
                        let bound = if i + 2 < ops.len() { ops[i + 2] } else { 0 };
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
                i += match primary {
                    TYPE_BST => 3,
                    TYPE_SEQ => {
                        if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR {
                            3
                        } else {
                            2
                        }
                    }
                    TYPE_BSQ => {
                        if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR {
                            4
                        } else {
                            3
                        }
                    }
                    _ => 2,
                };
            }
            OP_RTS | OP_DLC | OP_JEQ4 | OP_KOF | OP_MID => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    fields
}

// ---------------------------------------------------------------------------
// DynamicData CDR I/O helpers
// ---------------------------------------------------------------------------

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
    use cyclonedds_rust_sys::*;
    use std::ffi::c_void;

    let size = descriptor.size() as usize;
    let align = std::cmp::max(descriptor.align() as usize, 1);

    let layout = std::alloc::Layout::from_size_align(size, align)
        .map_err(|_| DdsError::BadParameter("invalid type layout for dynamic data".into()))?;
    let buf = unsafe { std::alloc::alloc_zeroed(layout) };
    if buf.is_null() {
        return Err(DdsError::OutOfMemory);
    }

    // Write the DynamicValue into the native buffer
    write_value_to_native(data.value(), buf, descriptor.ops(), 0);

    // Build CDR stream descriptor from topic descriptor ops
    let ops = descriptor.ops();
    let keys = descriptor.key_descriptors();

    let key_names: Vec<std::ffi::CString> = keys
        .iter()
        .map(|k| std::ffi::CString::new(k.name.as_str()).unwrap())
        .collect();
    let c_keys: Vec<dds_key_descriptor> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| dds_key_descriptor {
            m_name: key_names[i].as_ptr(),
            m_offset: k.offset,
            m_idx: k.index,
        })
        .collect();

    unsafe {
        let mut cdr_desc: dds_cdrstream_desc = std::mem::zeroed();
        dds_cdrstream_desc_init(
            &mut cdr_desc,
            &dds_cdrstream_default_allocator,
            descriptor.size(),
            descriptor.align(),
            descriptor.flagset(),
            ops.as_ptr(),
            if c_keys.is_empty() {
                std::ptr::null()
            } else {
                c_keys.as_ptr()
            },
            c_keys.len() as u32,
        );

        let mut os: dds_ostream_t = std::mem::zeroed();
        dds_ostream_init(
            &mut os,
            &dds_cdrstream_default_allocator,
            0,
            1, // XCDR1
        );

        let ok = dds_stream_write_sample(
            &mut os,
            &dds_cdrstream_default_allocator,
            buf as *const c_void,
            &cdr_desc,
        );

        let result = if ok {
            let len = os.m_index as usize;
            let mut bytes = vec![0u8; len];
            std::ptr::copy_nonoverlapping(os.m_buffer, bytes.as_mut_ptr(), len);
            Ok(bytes)
        } else {
            Err(DdsError::Unsupported("CDR serialization of dynamic data failed".into()))
        };

        dds_ostream_fini(&mut os, &dds_cdrstream_default_allocator);
        dds_cdrstream_desc_fini(&mut cdr_desc, &dds_cdrstream_default_allocator);

        // Free the native sample (dds_stream_free_sample handles any heap allocations)
        dds_stream_free_sample(
            buf as *mut c_void,
            &dds_cdrstream_default_allocator,
            ops.as_ptr(),
        );
        std::alloc::dealloc(buf, layout);

        result
    }
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
    use cyclonedds_rust_sys::*;
    use std::ffi::c_void;

    let size = descriptor.size() as usize;
    let align = std::cmp::max(descriptor.align() as usize, 1);

    let layout = std::alloc::Layout::from_size_align(size, align)
        .map_err(|_| DdsError::BadParameter("invalid type layout for dynamic data".into()))?;
    let buf = unsafe { std::alloc::alloc_zeroed(layout) };
    if buf.is_null() {
        return Err(DdsError::OutOfMemory);
    }

    let ops = descriptor.ops();
    let keys = descriptor.key_descriptors();

    let key_names: Vec<std::ffi::CString> = keys
        .iter()
        .map(|k| std::ffi::CString::new(k.name.as_str()).unwrap())
        .collect();
    let c_keys: Vec<dds_key_descriptor> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| dds_key_descriptor {
            m_name: key_names[i].as_ptr(),
            m_offset: k.offset,
            m_idx: k.index,
        })
        .collect();

    unsafe {
        let mut cdr_desc: dds_cdrstream_desc = std::mem::zeroed();
        dds_cdrstream_desc_init(
            &mut cdr_desc,
            &dds_cdrstream_default_allocator,
            descriptor.size(),
            descriptor.align(),
            descriptor.flagset(),
            ops.as_ptr(),
            if c_keys.is_empty() {
                std::ptr::null()
            } else {
                c_keys.as_ptr()
            },
            c_keys.len() as u32,
        );

        let mut is: dds_istream_t = std::mem::zeroed();
        dds_istream_init(
            &mut is,
            cdr_data.len() as u32,
            cdr_data.as_ptr() as *const c_void,
            1, // XCDR1
        );

        dds_stream_read_sample(
            &mut is,
            buf as *mut c_void,
            &dds_cdrstream_default_allocator,
            &cdr_desc,
        );

        // Extract values from the native buffer into a DynamicValue
        let value = read_value_from_native_public(buf, schema, ops, 0);

        dds_stream_free_sample(
            buf as *mut c_void,
            &dds_cdrstream_default_allocator,
            ops.as_ptr(),
        );
        std::alloc::dealloc(buf, layout);
        dds_cdrstream_desc_fini(&mut cdr_desc, &dds_cdrstream_default_allocator);
        dds_istream_fini(&mut is);

        Ok(crate::DynamicData::from_value(schema, value)?)
    }
}

// ---------------------------------------------------------------------------
// Native buffer <-> DynamicValue conversion
// ---------------------------------------------------------------------------

/// Write a `DynamicValue` into a native sample buffer at the given base offset,
/// following the ops array to determine field positions.
pub(crate) fn write_value_to_native(
    value: &crate::DynamicValue,
    base: *mut u8,
    ops: &[u32],
    ops_start: usize,
) {
    use crate::topic::*;
    use crate::dynamic_value::DynamicValue as DV;

    let struct_fields = match value {
        DV::Struct(fields) => fields,
        _ => return,
    };

    let mut i = ops_start;
    while i < ops.len() {
        let op = ops[i];
        let opcode = op & DDS_OP_MASK;

        match opcode {
            OP_ADR => {
                let primary = op & DDS_OP_TYPE_MASK;
                let offset = if i + 1 < ops.len() { ops[i + 1] as usize } else { 0 };
                let dst = unsafe { base.add(offset) };

                let field_name = format!("field_{}", (i - ops_start) / 2);
                let field_val = struct_fields.get(&field_name);

                if let Some(val) = field_val {
                    write_primitive_to_native(dst, val, primary, op);
                }

                // Advance
                let subtype = op & DDS_OP_SUBTYPE_MASK_CONST;
                i += match primary {
                    TYPE_BST => 3,
                    TYPE_SEQ => {
                        if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR {
                            3
                        } else {
                            2
                        }
                    }
                    TYPE_BSQ => {
                        if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR {
                            4
                        } else {
                            3
                        }
                    }
                    _ => 2,
                };
            }
            OP_RTS | OP_DLC => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
}

/// Write a single primitive value into the native buffer.
fn write_primitive_to_native(
    dst: *mut u8,
    val: &crate::dynamic_value::DynamicValue,
    primary_type: u32,
    _op: u32,
) {
    use crate::topic::*;
    use crate::dynamic_value::DynamicValue as DV;

    unsafe {
        match (primary_type, val) {
            (TYPE_1BY, DV::Bool(b)) => {
                let v: u8 = if *b { 1 } else { 0 };
                std::ptr::write(dst as *mut u8, v);
            }
            (TYPE_1BY, DV::Int8(v)) => {
                std::ptr::write(dst as *mut i8, *v);
            }
            (TYPE_1BY, DV::UInt8(v)) => {
                std::ptr::write(dst as *mut u8, *v);
            }
            (TYPE_1BY, DV::Byte(v)) => {
                std::ptr::write(dst as *mut u8, *v);
            }
            (TYPE_2BY, DV::Int16(v)) => {
                std::ptr::write(dst as *mut i16, *v);
            }
            (TYPE_2BY, DV::UInt16(v)) => {
                std::ptr::write(dst as *mut u16, *v);
            }
            (TYPE_4BY, DV::Int32(v)) => {
                std::ptr::write(dst as *mut i32, *v);
            }
            (TYPE_4BY, DV::UInt32(v)) => {
                std::ptr::write(dst as *mut u32, *v);
            }
            (TYPE_4BY, DV::Float32(v)) => {
                std::ptr::write(dst as *mut f32, *v);
            }
            (TYPE_4BY, DV::Enum { value }) => {
                std::ptr::write(dst as *mut i32, *value);
            }
            (TYPE_8BY, DV::Int64(v)) => {
                std::ptr::write(dst as *mut i64, *v);
            }
            (TYPE_8BY, DV::UInt64(v)) => {
                std::ptr::write(dst as *mut u64, *v);
            }
            (TYPE_8BY, DV::Float64(v)) => {
                std::ptr::write(dst as *mut f64, *v);
            }
            (TYPE_STR, DV::String(s)) => {
                // DDS string fields store a char* in the native buffer.
                // We need the pointer to outlive this function, so we leak it.
                // This is acceptable because dds_stream_free_sample will clean up
                // the native sample's heap allocations.
                let leaked = std::ffi::CString::new(s.as_str()).unwrap_or_default();
                std::ptr::write(dst as *mut *const i8, leaked.into_raw());
            }
            (TYPE_BST, DV::String(s)) => {
                // Bounded string: stored inline as char array up to the bound.
                // For simplicity, copy the bytes directly.
                let bytes = s.as_bytes();
                let len = bytes.len().min(255); // leave room for null terminator
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, len);
                *dst.add(len) = 0; // null terminator
            }
            _ => {
                // For types we don't handle, leave the zero-initialized buffer
            }
        }
    }
}

/// Read a `DynamicValue` from a native sample buffer, using the schema
/// to know which fields exist and the ops to find their offsets.
pub(crate) fn read_value_from_native_public(
    base: *mut u8,
    schema: &crate::DynamicTypeSchema,
    ops: &[u32],
    ops_start: usize,
) -> crate::DynamicValue {
    use crate::topic::*;
    use crate::dynamic_value::DynamicValue as DV;
    use std::collections::BTreeMap;

    let fields_schema = match schema {
        crate::DynamicTypeSchema::Struct { fields, .. } => fields,
        _ => return schema.default_value(),
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
                let offset = if i + 1 < ops.len() { ops[i + 1] as usize } else { 0 };
                let src = unsafe { base.add(offset) };

                let field_schema = fields_schema.get(field_idx);
                let val = read_primitive_from_native(src, primary, op, field_schema);

                let name = field_schema
                    .map(|f| f.name.clone())
                    .unwrap_or_else(|| format!("field_{}", field_idx));

                values.insert(name, val);
                field_idx += 1;

                let subtype = op & DDS_OP_SUBTYPE_MASK_CONST;
                i += match primary {
                    TYPE_BST => 3,
                    TYPE_SEQ => {
                        if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR {
                            3
                        } else {
                            2
                        }
                    }
                    TYPE_BSQ => {
                        if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR {
                            4
                        } else {
                            3
                        }
                    }
                    _ => 2,
                };
            }
            OP_RTS | OP_DLC => {
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    DV::Struct(values)
}

/// Read a single primitive value from the native buffer.
fn read_primitive_from_native(
    src: *mut u8,
    primary_type: u32,
    op: u32,
    field_schema: Option<&crate::dynamic_value::DynamicFieldSchema>,
) -> crate::dynamic_value::DynamicValue {
    use crate::topic::*;
    use crate::dynamic_value::DynamicValue as DV;

    unsafe {
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
                        crate::DynamicTypeSchema::Enum { .. } => {
                            DV::Enum { value: std::ptr::read(src as *const i32) }
                        }
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
                    DV::String(
                        CStr::from_ptr(ptr)
                            .to_string_lossy()
                            .into_owned(),
                    )
                }
            }
            TYPE_BST => {
                // Bounded string stored inline
                DV::String(
                    CStr::from_ptr(src as *const i8)
                        .to_string_lossy()
                        .into_owned(),
                )
            }
            _ => DV::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic_value::{DynamicTypeSchema, DynamicValue};
    use crate::dynamic_type::DynamicPrimitiveKind;

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
