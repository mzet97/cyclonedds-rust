//! Network type discovery via XTypes type lookup.
//!
//! This module provides the ability to discover type information from remote
//! participants by inspecting matched endpoints and resolving type identifiers
//! into full type definitions.

use crate::{
    dynamic_value::{DynamicFieldSchema, DynamicTypeSchema},
    xtypes::{FindScope, MatchedEndpoint, TopicDescriptor, TypeInfo},
    DataReader, DataWriter, DdsEntity, DdsError, DdsResult,
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
        self.topic_descriptor
            .create_topic(participant_entity, topic_name)
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
    let descriptor =
        type_info.create_topic_descriptor(participant_entity, FindScope::Global, timeout)?;
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
#[cfg_attr(feature = "tracing", tracing::instrument(skip(type_info)))]
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
                let subtype = (op >> 8) & DDS_OP_SUBTYPE_MASK;
                i += match primary {
                    TYPE_BST => 3,
                    TYPE_SEQ if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR => 3,
                    TYPE_SEQ => 2,
                    TYPE_BSQ if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR => 4,
                    TYPE_BSQ => 3,
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
    use crate::dynamic_value::DynamicValue as DV;
    use crate::topic::*;

    unsafe {
        match (primary_type, val) {
            (TYPE_1BY, DV::Bool(b)) => {
                let v: u8 = if *b { 1 } else { 0 };
                std::ptr::write(dst, v);
            }
            (TYPE_1BY, DV::Int8(v)) => {
                std::ptr::write(dst as *mut i8, *v);
            }
            (TYPE_1BY, DV::UInt8(v)) => {
                std::ptr::write(dst, *v);
            }
            (TYPE_1BY, DV::Byte(v)) => {
                std::ptr::write(dst, *v);
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
    use crate::dynamic_value::DynamicValue as DV;
    use crate::topic::*;
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
                let offset = if i + 1 < ops.len() {
                    ops[i + 1] as usize
                } else {
                    0
                };
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
                    TYPE_SEQ if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR => 3,
                    TYPE_SEQ => 2,
                    TYPE_BSQ if subtype == SUBTYPE_BST || subtype == SUBTYPE_STR => 4,
                    TYPE_BSQ => 3,
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
    use crate::dynamic_value::DynamicValue as DV;
    use crate::topic::*;

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
    use crate::dynamic_type::DynamicPrimitiveKind;
    use crate::dynamic_value::{DynamicTypeSchema, DynamicValue};

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
