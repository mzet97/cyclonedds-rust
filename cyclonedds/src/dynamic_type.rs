use crate::{
    dynamic_value::{
        DynamicBitmaskFieldSchema, DynamicEnumLiteralSchema, DynamicFieldSchema, DynamicTypeSchema,
        DynamicUnionCaseSchema,
    },
    entity::DdsEntity,
    error::check,
    xtypes::{FindScope, TopicDescriptor, TypeInfo},
    DdsError, DdsResult,
};
use cyclonedds_rust_sys::*;
use std::{ffi::CString, mem::ManuallyDrop};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicPrimitiveKind {
    Boolean,
    Byte,
    Int8,
    UInt8,
    Int16,
    UInt16,
    Int32,
    UInt32,
    Int64,
    UInt64,
    Float32,
    Float64,
    Char8,
    Char16,
}

impl DynamicPrimitiveKind {
    fn as_raw(self) -> dds_dynamic_type_kind_t {
        match self {
            Self::Boolean => dds_dynamic_type_kind_DDS_DYNAMIC_BOOLEAN,
            Self::Byte => dds_dynamic_type_kind_DDS_DYNAMIC_BYTE,
            Self::Int8 => dds_dynamic_type_kind_DDS_DYNAMIC_INT8,
            Self::UInt8 => dds_dynamic_type_kind_DDS_DYNAMIC_UINT8,
            Self::Int16 => dds_dynamic_type_kind_DDS_DYNAMIC_INT16,
            Self::UInt16 => dds_dynamic_type_kind_DDS_DYNAMIC_UINT16,
            Self::Int32 => dds_dynamic_type_kind_DDS_DYNAMIC_INT32,
            Self::UInt32 => dds_dynamic_type_kind_DDS_DYNAMIC_UINT32,
            Self::Int64 => dds_dynamic_type_kind_DDS_DYNAMIC_INT64,
            Self::UInt64 => dds_dynamic_type_kind_DDS_DYNAMIC_UINT64,
            Self::Float32 => dds_dynamic_type_kind_DDS_DYNAMIC_FLOAT32,
            Self::Float64 => dds_dynamic_type_kind_DDS_DYNAMIC_FLOAT64,
            Self::Char8 => dds_dynamic_type_kind_DDS_DYNAMIC_CHAR8,
            Self::Char16 => dds_dynamic_type_kind_DDS_DYNAMIC_CHAR16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicTypeExtensibility {
    Final,
    Appendable,
    Mutable,
}

impl DynamicTypeExtensibility {
    fn as_raw(self) -> dds_dynamic_type_extensibility {
        match self {
            Self::Final => dds_dynamic_type_extensibility_DDS_DYNAMIC_TYPE_EXT_FINAL,
            Self::Appendable => dds_dynamic_type_extensibility_DDS_DYNAMIC_TYPE_EXT_APPENDABLE,
            Self::Mutable => dds_dynamic_type_extensibility_DDS_DYNAMIC_TYPE_EXT_MUTABLE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicTypeAutoId {
    Sequential,
    Hash,
}

impl DynamicTypeAutoId {
    fn as_raw(self) -> dds_dynamic_type_autoid {
        match self {
            Self::Sequential => dds_dynamic_type_autoid_DDS_DYNAMIC_TYPE_AUTOID_SEQUENTIAL,
            Self::Hash => dds_dynamic_type_autoid_DDS_DYNAMIC_TYPE_AUTOID_HASH,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DynamicEnumLiteralValue {
    NextAvailable,
    Explicit(i32),
}

impl DynamicEnumLiteralValue {
    fn as_raw(self) -> dds_dynamic_enum_literal_value_t {
        match self {
            Self::NextAvailable => dds_dynamic_enum_literal_value_t {
                value_kind:
                    dds_dynamic_type_enum_value_kind_DDS_DYNAMIC_ENUM_LITERAL_VALUE_NEXT_AVAIL,
                value: 0,
            },
            Self::Explicit(value) => dds_dynamic_enum_literal_value_t {
                value_kind:
                    dds_dynamic_type_enum_value_kind_DDS_DYNAMIC_ENUM_LITERAL_VALUE_EXPLICIT,
                value,
            },
        }
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub struct DynamicTypeRefOwner {
    raw: dds_dynamic_type_t,
    schema: DynamicTypeSchema,
}

impl Drop for DynamicTypeRefOwner {
    fn drop(&mut self) {
        unsafe {
            let _ = dds_dynamic_type_unref(&mut self.raw);
        }
    }
}

#[derive(Debug)]
pub enum DynamicTypeSpec {
    Primitive(DynamicPrimitiveKind),
    Dynamic(DynamicTypeRefOwner),
}

impl DynamicTypeSpec {
    pub fn primitive(kind: DynamicPrimitiveKind) -> Self {
        Self::Primitive(kind)
    }

    pub fn from_type(dynamic_type: &DynamicType) -> Self {
        let mut raw = dynamic_type.raw;
        let referenced = unsafe { dds_dynamic_type_ref(&mut raw) };
        Self::Dynamic(DynamicTypeRefOwner {
            raw: referenced,
            schema: dynamic_type.schema.clone(),
        })
    }

    fn into_raw(self) -> dds_dynamic_type_spec_t {
        match self {
            Self::Primitive(kind) => dds_dynamic_type_spec_t {
                kind: dds_dynamic_type_spec_kind_DDS_DYNAMIC_TYPE_KIND_PRIMITIVE,
                type_: dds_dynamic_type_spec__bindgen_ty_1 {
                    primitive: kind.as_raw(),
                },
            },
            Self::Dynamic(owner) => {
                let owner = ManuallyDrop::new(owner);
                dds_dynamic_type_spec_t {
                    kind: dds_dynamic_type_spec_kind_DDS_DYNAMIC_TYPE_KIND_DEFINITION,
                    type_: dds_dynamic_type_spec__bindgen_ty_1 { type_: owner.raw },
                }
            }
        }
    }

    fn schema_clone(&self) -> DynamicTypeSchema {
        match self {
            Self::Primitive(kind) => DynamicTypeSchema::Primitive(*kind),
            Self::Dynamic(owner) => owner.schema.clone(),
        }
    }
}

#[derive(Debug)]
pub struct DynamicTypeBuilder {
    kind: dds_dynamic_type_kind_t,
    name: Option<String>,
    base_type: Option<DynamicTypeSpec>,
    discriminator_type: Option<DynamicTypeSpec>,
    bounds: Vec<u32>,
    element_type: Option<DynamicTypeSpec>,
    key_element_type: Option<DynamicTypeSpec>,
    extensibility: Option<DynamicTypeExtensibility>,
    autoid: Option<DynamicTypeAutoId>,
    nested: Option<bool>,
    bit_bound: Option<u16>,
}

impl DynamicTypeBuilder {
    pub fn structure(name: impl Into<String>) -> Self {
        Self::new(
            dds_dynamic_type_kind_DDS_DYNAMIC_STRUCTURE,
            Some(name.into()),
        )
    }

    pub fn enumeration(name: impl Into<String>) -> Self {
        Self::new(
            dds_dynamic_type_kind_DDS_DYNAMIC_ENUMERATION,
            Some(name.into()),
        )
    }

    pub fn bitmask(name: impl Into<String>) -> Self {
        Self::new(dds_dynamic_type_kind_DDS_DYNAMIC_BITMASK, Some(name.into()))
    }

    pub fn alias(name: impl Into<String>, base_type: DynamicTypeSpec) -> Self {
        Self::new(dds_dynamic_type_kind_DDS_DYNAMIC_ALIAS, Some(name.into())).base_type(base_type)
    }

    pub fn string8(bound: Option<u32>) -> Self {
        let mut builder = Self::new(dds_dynamic_type_kind_DDS_DYNAMIC_STRING8, None);
        if let Some(bound) = bound {
            builder.bounds.push(bound);
        }
        builder
    }

    pub fn bounded_string8(bound: u32) -> Self {
        Self::string8(Some(bound))
    }

    pub fn unbounded_string8() -> Self {
        Self::string8(None)
    }

    pub fn sequence(
        name: impl Into<String>,
        element_type: DynamicTypeSpec,
        bound: Option<u32>,
    ) -> Self {
        let mut builder = Self::new(
            dds_dynamic_type_kind_DDS_DYNAMIC_SEQUENCE,
            Some(name.into()),
        )
        .element_type(element_type);
        if let Some(bound) = bound {
            builder.bounds.push(bound);
        }
        builder
    }

    pub fn array(name: impl Into<String>, element_type: DynamicTypeSpec, bounds: Vec<u32>) -> Self {
        Self::new(dds_dynamic_type_kind_DDS_DYNAMIC_ARRAY, Some(name.into()))
            .element_type(element_type)
            .bounds(bounds)
    }

    pub fn unbounded_sequence(name: impl Into<String>, element_type: DynamicTypeSpec) -> Self {
        Self::sequence(name, element_type, None)
    }

    pub fn bounded_sequence(
        name: impl Into<String>,
        element_type: DynamicTypeSpec,
        bound: u32,
    ) -> Self {
        Self::sequence(name, element_type, Some(bound))
    }

    pub fn union(name: impl Into<String>, discriminator_type: DynamicTypeSpec) -> Self {
        Self::new(dds_dynamic_type_kind_DDS_DYNAMIC_UNION, Some(name.into()))
            .discriminator_type(discriminator_type)
    }

    pub fn map(
        name: impl Into<String>,
        key_element_type: DynamicTypeSpec,
        element_type: DynamicTypeSpec,
        bound: Option<u32>,
    ) -> Self {
        let mut builder = Self::new(dds_dynamic_type_kind_DDS_DYNAMIC_MAP, Some(name.into()))
            .key_element_type(key_element_type)
            .element_type(element_type);
        if let Some(bound) = bound {
            builder.bounds.push(bound);
        }
        builder
    }

    pub fn bounded_map(
        name: impl Into<String>,
        key_element_type: DynamicTypeSpec,
        element_type: DynamicTypeSpec,
        bound: u32,
    ) -> Self {
        Self::map(name, key_element_type, element_type, Some(bound))
    }

    pub fn unbounded_map(
        name: impl Into<String>,
        key_element_type: DynamicTypeSpec,
        element_type: DynamicTypeSpec,
    ) -> Self {
        Self::map(name, key_element_type, element_type, None)
    }

    fn new(kind: dds_dynamic_type_kind_t, name: Option<String>) -> Self {
        Self {
            kind,
            name,
            base_type: None,
            discriminator_type: None,
            bounds: Vec::new(),
            element_type: None,
            key_element_type: None,
            extensibility: None,
            autoid: None,
            nested: None,
            bit_bound: None,
        }
    }

    pub fn base_type(mut self, base_type: DynamicTypeSpec) -> Self {
        self.base_type = Some(base_type);
        self
    }

    pub fn discriminator_type(mut self, discriminator_type: DynamicTypeSpec) -> Self {
        self.discriminator_type = Some(discriminator_type);
        self
    }

    pub fn bounds(mut self, bounds: Vec<u32>) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn element_type(mut self, element_type: DynamicTypeSpec) -> Self {
        self.element_type = Some(element_type);
        self
    }

    pub fn key_element_type(mut self, key_element_type: DynamicTypeSpec) -> Self {
        self.key_element_type = Some(key_element_type);
        self
    }

    pub fn final_extensibility(self) -> Self {
        self.extensibility(DynamicTypeExtensibility::Final)
    }

    pub fn appendable(self) -> Self {
        self.extensibility(DynamicTypeExtensibility::Appendable)
    }

    pub fn mutable_extensibility(self) -> Self {
        self.extensibility(DynamicTypeExtensibility::Mutable)
    }

    pub fn extensibility(mut self, extensibility: DynamicTypeExtensibility) -> Self {
        self.extensibility = Some(extensibility);
        self
    }

    pub fn autoid_sequential(self) -> Self {
        self.autoid(DynamicTypeAutoId::Sequential)
    }

    pub fn autoid_hash(self) -> Self {
        self.autoid(DynamicTypeAutoId::Hash)
    }

    pub fn autoid(mut self, autoid: DynamicTypeAutoId) -> Self {
        self.autoid = Some(autoid);
        self
    }

    pub fn nested(mut self, is_nested: bool) -> Self {
        self.nested = Some(is_nested);
        self
    }

    pub fn bit_bound(mut self, bit_bound: u16) -> Self {
        self.bit_bound = Some(bit_bound);
        self
    }

    fn to_schema(&self) -> DynamicTypeSchema {
        match self.kind {
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_STRUCTURE => DynamicTypeSchema::Struct {
                name: self.name.clone().unwrap_or_default(),
                base: self.base_type.as_ref().map(|b| Box::new(b.schema_clone())),
                fields: Vec::new(),
                extensibility: self.extensibility,
                autoid: self.autoid,
                nested: self.nested.unwrap_or(false),
            },
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_ENUMERATION => DynamicTypeSchema::Enum {
                name: self.name.clone().unwrap_or_default(),
                literals: Vec::new(),
                bit_bound: self.bit_bound,
            },
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_BITMASK => DynamicTypeSchema::Bitmask {
                name: self.name.clone().unwrap_or_default(),
                fields: Vec::new(),
                bit_bound: self.bit_bound,
            },
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_ALIAS => DynamicTypeSchema::Alias {
                name: self.name.clone().unwrap_or_default(),
                target: Box::new(
                    self.base_type
                        .as_ref()
                        .expect("alias requires base type")
                        .schema_clone(),
                ),
            },
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_ARRAY => DynamicTypeSchema::Array {
                name: self.name.clone().unwrap_or_default(),
                bounds: self.bounds.clone(),
                element: Box::new(
                    self.element_type
                        .as_ref()
                        .expect("array requires element type")
                        .schema_clone(),
                ),
            },
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_SEQUENCE => DynamicTypeSchema::Sequence {
                name: self.name.clone().unwrap_or_default(),
                bound: self.bounds.first().copied(),
                element: Box::new(
                    self.element_type
                        .as_ref()
                        .expect("sequence requires element type")
                        .schema_clone(),
                ),
            },
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_STRING8 => DynamicTypeSchema::String {
                bound: self.bounds.first().copied(),
            },
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_UNION => DynamicTypeSchema::Union {
                name: self.name.clone().unwrap_or_default(),
                discriminator: Box::new(
                    self.discriminator_type
                        .as_ref()
                        .expect("union requires discriminator type")
                        .schema_clone(),
                ),
                cases: Vec::new(),
                autoid: self.autoid,
                nested: self.nested.unwrap_or(false),
            },
            x if x == dds_dynamic_type_kind_DDS_DYNAMIC_MAP => DynamicTypeSchema::Map {
                name: self.name.clone().unwrap_or_default(),
                bound: self.bounds.first().copied(),
                key: Box::new(
                    self.key_element_type
                        .as_ref()
                        .expect("map requires key type")
                        .schema_clone(),
                ),
                value: Box::new(
                    self.element_type
                        .as_ref()
                        .expect("map requires value type")
                        .schema_clone(),
                ),
            },
            _ => panic!("unsupported dynamic builder kind for schema"),
        }
    }

    pub fn build<E: DdsEntity>(self, entity: &E) -> DdsResult<DynamicType> {
        DynamicType::create(entity.entity(), self)
    }
}

#[derive(Debug)]
pub struct DynamicMemberBuilder {
    name: String,
    id: u32,
    type_spec: DynamicTypeSpec,
    index: u32,
    labels: Vec<i32>,
    default_label: bool,
    is_key: bool,
    is_optional: bool,
    is_external: bool,
    is_must_understand: bool,
    hash_id_name: Option<String>,
}

impl DynamicMemberBuilder {
    pub fn new(name: impl Into<String>, type_spec: DynamicTypeSpec) -> Self {
        Self {
            name: name.into(),
            id: DDS_DYNAMIC_MEMBER_ID_AUTO,
            type_spec,
            index: DDS_DYNAMIC_MEMBER_INDEX_END,
            labels: Vec::new(),
            default_label: false,
            is_key: false,
            is_optional: false,
            is_external: false,
            is_must_understand: false,
            hash_id_name: None,
        }
    }

    pub fn primitive(name: impl Into<String>, kind: DynamicPrimitiveKind) -> Self {
        Self::new(name, DynamicTypeSpec::primitive(kind))
    }

    pub fn id(mut self, id: u32) -> Self {
        self.id = id;
        self
    }

    pub fn at_index(mut self, index: u32) -> Self {
        self.index = index;
        self
    }

    pub fn at_start(self) -> Self {
        self.at_index(DDS_DYNAMIC_MEMBER_INDEX_START)
    }

    pub fn first(self) -> Self {
        self.at_start()
    }

    pub fn at_end(self) -> Self {
        self.at_index(DDS_DYNAMIC_MEMBER_INDEX_END)
    }

    pub fn last(self) -> Self {
        self.at_end()
    }

    pub fn labels(mut self, labels: &[i32]) -> Self {
        self.labels = labels.to_vec();
        self
    }

    pub fn default_label(mut self) -> Self {
        self.default_label = true;
        self.labels.clear();
        self
    }

    pub fn union_labels(self, labels: &[i32]) -> Self {
        self.labels(labels)
    }

    pub fn union_default(self) -> Self {
        self.default_label()
    }

    pub fn key(mut self) -> Self {
        self.is_key = true;
        self
    }

    pub fn optional(mut self) -> Self {
        self.is_optional = true;
        self
    }

    pub fn external(mut self) -> Self {
        self.is_external = true;
        self
    }

    pub fn must_understand(mut self) -> Self {
        self.is_must_understand = true;
        self
    }

    pub fn hash_id(mut self, hash_id_name: impl Into<String>) -> Self {
        self.hash_id_name = Some(hash_id_name.into());
        self
    }
}

#[derive(Debug)]
pub struct DynamicType {
    raw: dds_dynamic_type_t,
    schema: DynamicTypeSchema,
}

impl DynamicType {
    pub fn create(entity: dds_entity_t, builder: DynamicTypeBuilder) -> DdsResult<Self> {
        let schema = builder.to_schema();
        let DynamicTypeBuilder {
            kind,
            name,
            base_type,
            discriminator_type,
            bounds,
            element_type,
            key_element_type,
            extensibility,
            autoid,
            nested,
            bit_bound,
        } = builder;

        let name_cstr =
            match name {
                Some(name) => Some(CString::new(name).map_err(|_| {
                    DdsError::BadParameter("dynamic type name contains null".into())
                })?),
                None => None,
            };

        let descriptor = dds_dynamic_type_descriptor_t {
            kind,
            name: name_cstr
                .as_ref()
                .map_or(std::ptr::null(), |name| name.as_ptr()),
            base_type: base_type.map(DynamicTypeSpec::into_raw).unwrap_or_default(),
            discriminator_type: discriminator_type
                .map(DynamicTypeSpec::into_raw)
                .unwrap_or_default(),
            num_bounds: bounds.len() as u32,
            bounds: if bounds.is_empty() {
                std::ptr::null()
            } else {
                bounds.as_ptr()
            },
            element_type: element_type
                .map(DynamicTypeSpec::into_raw)
                .unwrap_or_default(),
            key_element_type: key_element_type
                .map(DynamicTypeSpec::into_raw)
                .unwrap_or_default(),
        };

        let raw = unsafe { dds_dynamic_type_create(entity, descriptor) };
        check(raw.ret)?;
        if raw.x.iter().all(|ptr| ptr.is_null()) {
            return Err(DdsError::Other(
                "CycloneDDS returned null dynamic type handle".into(),
            ));
        }
        let mut dynamic_type = Self { raw, schema };
        if let Some(extensibility) = extensibility {
            dynamic_type.set_extensibility(extensibility)?;
        }
        if let Some(autoid) = autoid {
            dynamic_type.set_autoid(autoid)?;
        }
        if let Some(is_nested) = nested {
            dynamic_type.set_nested(is_nested)?;
        }
        if let Some(bit_bound) = bit_bound {
            dynamic_type.set_bit_bound(bit_bound)?;
        }
        Ok(dynamic_type)
    }

    pub fn duplicate(&self) -> DdsResult<Self> {
        let raw = unsafe { dds_dynamic_type_dup(&self.raw) };
        check(raw.ret)?;
        if raw.x.iter().all(|ptr| ptr.is_null()) {
            return Err(DdsError::Other(
                "CycloneDDS returned null duplicated dynamic type".into(),
            ));
        }
        Ok(Self {
            raw,
            schema: self.schema.clone(),
        })
    }

    pub fn as_spec(&self) -> DynamicTypeSpec {
        DynamicTypeSpec::from_type(self)
    }

    pub fn schema(&self) -> &DynamicTypeSchema {
        &self.schema
    }

    pub fn set_extensibility(&mut self, extensibility: DynamicTypeExtensibility) -> DdsResult<()> {
        unsafe {
            check(dds_dynamic_type_set_extensibility(
                &mut self.raw,
                extensibility.as_raw(),
            ))?;
        }
        match &mut self.schema {
            DynamicTypeSchema::Struct {
                extensibility: current,
                ..
            } => *current = Some(extensibility),
            DynamicTypeSchema::Union { .. } => {}
            DynamicTypeSchema::Enum { .. } | DynamicTypeSchema::Bitmask { .. } => {}
            _ => {}
        }
        Ok(())
    }

    pub fn set_autoid(&mut self, autoid: DynamicTypeAutoId) -> DdsResult<()> {
        unsafe {
            check(dds_dynamic_type_set_autoid(&mut self.raw, autoid.as_raw()))?;
        }
        match &mut self.schema {
            DynamicTypeSchema::Struct {
                autoid: current, ..
            } => *current = Some(autoid),
            DynamicTypeSchema::Union {
                autoid: current, ..
            } => *current = Some(autoid),
            _ => {}
        }
        Ok(())
    }

    pub fn set_nested(&mut self, is_nested: bool) -> DdsResult<()> {
        unsafe {
            check(dds_dynamic_type_set_nested(&mut self.raw, is_nested))?;
        }
        match &mut self.schema {
            DynamicTypeSchema::Struct { nested, .. } => *nested = is_nested,
            DynamicTypeSchema::Union { nested, .. } => *nested = is_nested,
            _ => {}
        }
        Ok(())
    }

    pub fn set_bit_bound(&mut self, bit_bound: u16) -> DdsResult<()> {
        unsafe {
            check(dds_dynamic_type_set_bit_bound(&mut self.raw, bit_bound))?;
        }
        match &mut self.schema {
            DynamicTypeSchema::Enum {
                bit_bound: current, ..
            } => *current = Some(bit_bound),
            DynamicTypeSchema::Bitmask {
                bit_bound: current, ..
            } => *current = Some(bit_bound),
            _ => {}
        }
        Ok(())
    }

    pub fn add_member(&mut self, member: DynamicMemberBuilder) -> DdsResult<()> {
        let DynamicMemberBuilder {
            name,
            id,
            type_spec,
            index,
            labels,
            default_label,
            is_key,
            is_optional,
            is_external,
            is_must_understand,
            hash_id_name,
        } = member;
        let name_cstr = CString::new(name.clone())
            .map_err(|_| DdsError::BadParameter("dynamic member name contains null".into()))?;
        let value_schema = type_spec.schema_clone();
        let descriptor = dds_dynamic_member_descriptor_t {
            name: name_cstr.as_ptr(),
            id,
            type_: type_spec.into_raw(),
            default_value: std::ptr::null_mut(),
            index,
            num_labels: labels.len() as u32,
            labels: if labels.is_empty() {
                std::ptr::null_mut()
            } else {
                labels.as_ptr().cast_mut()
            },
            default_label,
        };
        unsafe { check(dds_dynamic_type_add_member(&mut self.raw, descriptor))? };
        let needs_member_id =
            is_key || is_optional || is_external || is_must_understand || hash_id_name.is_some();
        if needs_member_id && id == DDS_DYNAMIC_MEMBER_ID_AUTO {
            return Err(DdsError::BadParameter(
                "dynamic member properties require an explicit member id".into(),
            ));
        }
        match &mut self.schema {
            DynamicTypeSchema::Struct { fields, .. } => fields.push(DynamicFieldSchema {
                name,
                member_id: id,
                value: value_schema,
                optional: is_optional,
                key: is_key,
                external: is_external,
                must_understand: is_must_understand,
                hash_id_name: hash_id_name.clone(),
            }),
            DynamicTypeSchema::Union { cases, .. } => cases.push(DynamicUnionCaseSchema {
                name,
                member_id: id,
                value: value_schema,
                labels: labels.clone(),
                default: default_label,
                external: is_external,
                hash_id_name: hash_id_name.clone(),
            }),
            _ => {}
        }
        if is_key {
            self.set_member_key(id, true)?;
        }
        if is_optional {
            self.set_member_optional(id, true)?;
        }
        if is_external {
            self.set_member_external(id, true)?;
        }
        if is_must_understand {
            self.set_member_must_understand(id, true)?;
        }
        if let Some(hash_id_name) = hash_id_name {
            self.set_member_hash_id(id, &hash_id_name)?;
        }
        Ok(())
    }

    pub fn add_enum_literal(
        &mut self,
        name: &str,
        value: DynamicEnumLiteralValue,
        is_default: bool,
    ) -> DdsResult<()> {
        let name_cstr = CString::new(name)
            .map_err(|_| DdsError::BadParameter("enum literal name contains null".into()))?;
        unsafe {
            check(dds_dynamic_type_add_enum_literal(
                &mut self.raw,
                name_cstr.as_ptr(),
                value.as_raw(),
                is_default,
            ))?;
        }
        if let DynamicTypeSchema::Enum { literals, .. } = &mut self.schema {
            let resolved = match value {
                DynamicEnumLiteralValue::Explicit(v) => v,
                DynamicEnumLiteralValue::NextAvailable => {
                    literals.iter().map(|l| l.value).max().unwrap_or(-1) + 1
                }
            };
            literals.push(DynamicEnumLiteralSchema {
                name: name.to_string(),
                value: resolved,
                default: is_default,
            });
        }
        Ok(())
    }

    pub fn add_bitmask_field(&mut self, name: &str, position: Option<u16>) -> DdsResult<()> {
        let name_cstr = CString::new(name)
            .map_err(|_| DdsError::BadParameter("bitmask field name contains null".into()))?;
        unsafe {
            check(dds_dynamic_type_add_bitmask_field(
                &mut self.raw,
                name_cstr.as_ptr(),
                position.unwrap_or(DDS_DYNAMIC_BITMASK_POSITION_AUTO as u16),
            ))?;
        }
        if let DynamicTypeSchema::Bitmask { fields, .. } = &mut self.schema {
            let resolved = position.unwrap_or_else(|| {
                fields
                    .iter()
                    .map(|f| f.position)
                    .max()
                    .unwrap_or(u16::MAX)
                    .wrapping_add(1)
            });
            fields.push(DynamicBitmaskFieldSchema {
                name: name.to_string(),
                position: resolved,
            });
        }
        Ok(())
    }

    pub fn set_member_key(&mut self, member_id: u32, is_key: bool) -> DdsResult<()> {
        unsafe {
            check(dds_dynamic_member_set_key(&mut self.raw, member_id, is_key))?;
        }
        if let DynamicTypeSchema::Struct { fields, .. } = &mut self.schema {
            if let Some(field) = fields.iter_mut().find(|f| f.member_id == member_id) {
                field.key = is_key;
            }
        }
        Ok(())
    }

    pub fn set_member_optional(&mut self, member_id: u32, is_optional: bool) -> DdsResult<()> {
        unsafe {
            check(dds_dynamic_member_set_optional(
                &mut self.raw,
                member_id,
                is_optional,
            ))?;
        }
        if let DynamicTypeSchema::Struct { fields, .. } = &mut self.schema {
            if let Some(field) = fields.iter_mut().find(|f| f.member_id == member_id) {
                field.optional = is_optional;
            }
        }
        Ok(())
    }

    pub fn set_member_external(&mut self, member_id: u32, is_external: bool) -> DdsResult<()> {
        unsafe {
            check(dds_dynamic_member_set_external(
                &mut self.raw,
                member_id,
                is_external,
            ))?;
        }
        match &mut self.schema {
            DynamicTypeSchema::Struct { fields, .. } => {
                if let Some(field) = fields.iter_mut().find(|f| f.member_id == member_id) {
                    field.external = is_external;
                }
            }
            DynamicTypeSchema::Union { cases, .. } => {
                if let Some(case) = cases.iter_mut().find(|c| c.member_id == member_id) {
                    case.external = is_external;
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn set_member_hash_id(&mut self, member_id: u32, hash_name: &str) -> DdsResult<()> {
        let hash_name_c = CString::new(hash_name)
            .map_err(|_| DdsError::BadParameter("hash member name contains null".into()))?;
        unsafe {
            check(dds_dynamic_member_set_hashid(
                &mut self.raw,
                member_id,
                hash_name_c.as_ptr(),
            ))?;
        }
        match &mut self.schema {
            DynamicTypeSchema::Struct { fields, .. } => {
                if let Some(field) = fields.iter_mut().find(|f| f.member_id == member_id) {
                    field.hash_id_name = Some(hash_name.to_string());
                }
            }
            DynamicTypeSchema::Union { cases, .. } => {
                if let Some(case) = cases.iter_mut().find(|c| c.member_id == member_id) {
                    case.hash_id_name = Some(hash_name.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn set_member_must_understand(
        &mut self,
        member_id: u32,
        is_must_understand: bool,
    ) -> DdsResult<()> {
        unsafe {
            check(dds_dynamic_member_set_must_understand(
                &mut self.raw,
                member_id,
                is_must_understand,
            ))?;
        }
        if let DynamicTypeSchema::Struct { fields, .. } = &mut self.schema {
            if let Some(field) = fields.iter_mut().find(|f| f.member_id == member_id) {
                field.must_understand = is_must_understand;
            }
        }
        Ok(())
    }

    pub fn register(&mut self) -> DdsResult<TypeInfo> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            check(dds_dynamic_type_register(&mut self.raw, &mut ptr))?;
        }
        if ptr.is_null() {
            return Err(DdsError::Other(
                "CycloneDDS returned null typeinfo for registered dynamic type".into(),
            ));
        }
        Ok(TypeInfo::from_raw(ptr))
    }

    pub fn register_type_info(&mut self) -> DdsResult<TypeInfo> {
        self.register()
    }

    pub fn register_topic_descriptor<E: DdsEntity>(
        &mut self,
        participant: &E,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<TopicDescriptor> {
        let type_info = self.register()?;
        type_info.create_topic_descriptor(participant.entity(), scope, timeout)
    }

    pub fn register_topic<E: DdsEntity>(
        &mut self,
        participant: &E,
        scope: FindScope,
        timeout: dds_duration_t,
        name: &str,
    ) -> DdsResult<crate::UntypedTopic> {
        let descriptor = self.register_topic_descriptor(participant, scope, timeout)?;
        descriptor.create_topic(participant.entity(), name)
    }
}

impl Drop for DynamicType {
    fn drop(&mut self) {
        unsafe {
            let _ = dds_dynamic_type_unref(&mut self.raw);
        }
    }
}
