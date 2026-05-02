use crate::{
    dynamic_type::{DynamicPrimitiveKind, DynamicTypeAutoId, DynamicTypeExtensibility},
    DdsError, DdsResult,
};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// DynamicData  (Part 4.1 — runtime-typed data value for DDS I/O)
// ---------------------------------------------------------------------------

/// A runtime-typed data value that can be read from and written to DDS.
///
/// `DynamicData` wraps a [`DynamicValue`] tree together with its
/// [`DynamicTypeSchema`], providing field-level access by name or index.
/// It can be serialized to CDR bytes for writing through a DDS writer
/// and deserialized from CDR bytes read through a DDS reader.
///
/// # Lifecycle
///
/// 1. Build a [`DynamicType`] via [`DynamicTypeBuilder`](crate::DynamicTypeBuilder).
/// 2. Register it with a participant to obtain a [`TopicDescriptor`].
/// 3. Create a `DynamicData` from the schema.
/// 4. Set field values, then serialize to CDR for writing.
/// 5. On the reader side, deserialize CDR bytes back into `DynamicData`.
///
/// [`DynamicType`]: crate::DynamicType
/// [`TopicDescriptor`]: crate::TopicDescriptor
pub struct DynamicData {
    value: DynamicValue,
    schema: DynamicTypeSchema,
}

impl DynamicData {
    /// Create a new `DynamicData` initialized to default values for the given schema.
    pub fn new(schema: &DynamicTypeSchema) -> Self {
        Self {
            value: schema.default_value(),
            schema: schema.clone(),
        }
    }

    /// Create a `DynamicData` from an existing `DynamicValue`, validating it
    /// against the provided schema.
    pub fn from_value(schema: &DynamicTypeSchema, value: DynamicValue) -> DdsResult<Self> {
        value.validate_against(schema)?;
        Ok(Self {
            value,
            schema: schema.clone(),
        })
    }

    /// Get a reference to the underlying value tree.
    pub fn value(&self) -> &DynamicValue {
        &self.value
    }

    /// Get a mutable reference to the underlying value tree.
    ///
    /// The caller is responsible for ensuring the value remains valid
    /// for the schema after mutation.
    pub fn value_mut(&mut self) -> &mut DynamicValue {
        &mut self.value
    }

    /// Get the schema describing this data's type.
    pub fn schema(&self) -> &DynamicTypeSchema {
        &self.schema
    }

    // ── Primitive field access by field name (struct types) ──

    /// Get a boolean field value by name.
    pub fn get_bool(&self, name: &str) -> DdsResult<bool> {
        match self.value.field(name) {
            Some(DynamicValue::Bool(b)) => Ok(*b),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not a bool",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get an i32 field value by name.
    pub fn get_i32(&self, name: &str) -> DdsResult<i32> {
        match self.value.field(name) {
            Some(DynamicValue::Int32(v)) => Ok(*v),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not an i32",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get a u32 field value by name.
    pub fn get_u32(&self, name: &str) -> DdsResult<u32> {
        match self.value.field(name) {
            Some(DynamicValue::UInt32(v)) => Ok(*v),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not a u32",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get an i64 field value by name.
    pub fn get_i64(&self, name: &str) -> DdsResult<i64> {
        match self.value.field(name) {
            Some(DynamicValue::Int64(v)) => Ok(*v),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not an i64",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get a u64 field value by name.
    pub fn get_u64(&self, name: &str) -> DdsResult<u64> {
        match self.value.field(name) {
            Some(DynamicValue::UInt64(v)) => Ok(*v),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not a u64",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get an f32 field value by name.
    pub fn get_f32(&self, name: &str) -> DdsResult<f32> {
        match self.value.field(name) {
            Some(DynamicValue::Float32(v)) => Ok(*v),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not an f32",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get an f64 field value by name.
    pub fn get_f64(&self, name: &str) -> DdsResult<f64> {
        match self.value.field(name) {
            Some(DynamicValue::Float64(v)) => Ok(*v),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not an f64",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get a string field value by name.
    pub fn get_string(&self, name: &str) -> DdsResult<String> {
        match self.value.field(name) {
            Some(DynamicValue::String(s)) => Ok(s.clone()),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not a string",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get an enum field value by name. Returns the integer value.
    pub fn get_enum(&self, name: &str) -> DdsResult<i32> {
        match self.value.field(name) {
            Some(DynamicValue::Enum { value }) => Ok(*value),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not an enum",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get a bitmask field value by name.
    pub fn get_bitmask(&self, name: &str) -> DdsResult<u64> {
        match self.value.field(name) {
            Some(DynamicValue::Bitmask(v)) => Ok(*v),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not a bitmask",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get a nested struct field by name, returned as a new `DynamicData`.
    pub fn get_struct(&self, name: &str) -> DdsResult<DynamicData> {
        let field_schema = self.schema.field(name).ok_or_else(|| {
            DdsError::BadParameter(format!("field '{}' not found in schema", name))
        })?;
        match self.value.field(name) {
            Some(DynamicValue::Struct(map)) => Ok(DynamicData {
                value: DynamicValue::Struct(map.clone()),
                schema: field_schema.value.clone(),
            }),
            Some(DynamicValue::Null) if field_schema.optional => Err(DdsError::BadParameter(
                format!("optional field '{}' is not set", name),
            )),
            Some(v) => Err(DdsError::BadParameter(format!(
                "field '{}' is {:?}, expected struct",
                name, v
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    /// Get a sequence field by name, returning each element as a `DynamicValue`.
    pub fn get_sequence(&self, name: &str) -> DdsResult<Vec<DynamicValue>> {
        match self.value.field(name) {
            Some(DynamicValue::Sequence(elems)) => Ok(elems.clone()),
            Some(DynamicValue::Array(elems)) => Ok(elems.clone()),
            Some(_) => Err(DdsError::BadParameter(format!(
                "field '{}' is not a sequence",
                name
            ))),
            None => Err(DdsError::BadParameter(format!(
                "field '{}' not found",
                name
            ))),
        }
    }

    // ── Primitive field setters by name ──

    /// Set a boolean field value by name.
    pub fn set_bool(&mut self, name: &str, value: bool) -> DdsResult<()> {
        self.set_field(name, DynamicValue::Bool(value))
    }

    /// Set an i32 field value by name.
    pub fn set_i32(&mut self, name: &str, value: i32) -> DdsResult<()> {
        self.set_field(name, DynamicValue::Int32(value))
    }

    /// Set a u32 field value by name.
    pub fn set_u32(&mut self, name: &str, value: u32) -> DdsResult<()> {
        self.set_field(name, DynamicValue::UInt32(value))
    }

    /// Set an i64 field value by name.
    pub fn set_i64(&mut self, name: &str, value: i64) -> DdsResult<()> {
        self.set_field(name, DynamicValue::Int64(value))
    }

    /// Set a u64 field value by name.
    pub fn set_u64(&mut self, name: &str, value: u64) -> DdsResult<()> {
        self.set_field(name, DynamicValue::UInt64(value))
    }

    /// Set an f32 field value by name.
    pub fn set_f32(&mut self, name: &str, value: f32) -> DdsResult<()> {
        self.set_field(name, DynamicValue::Float32(value))
    }

    /// Set an f64 field value by name.
    pub fn set_f64(&mut self, name: &str, value: f64) -> DdsResult<()> {
        self.set_field(name, DynamicValue::Float64(value))
    }

    /// Set a string field value by name.
    pub fn set_string(&mut self, name: &str, value: impl Into<String>) -> DdsResult<()> {
        self.set_field(name, DynamicValue::String(value.into()))
    }

    /// Set an enum field value by name.
    pub fn set_enum(&mut self, name: &str, value: i32) -> DdsResult<()> {
        self.set_field(name, DynamicValue::Enum { value })
    }

    /// Set a bitmask field value by name.
    pub fn set_bitmask(&mut self, name: &str, value: u64) -> DdsResult<()> {
        self.set_field(name, DynamicValue::Bitmask(value))
    }

    /// Set a field to a null (unset optional) value.
    pub fn set_null(&mut self, name: &str) -> DdsResult<()> {
        let field = self.schema.field(name).ok_or_else(|| {
            DdsError::BadParameter(format!("field '{}' not found in schema", name))
        })?;
        if !field.optional {
            return Err(DdsError::BadParameter(format!(
                "field '{}' is not optional, cannot set to null",
                name
            )));
        }
        self.value.set_field(&self.schema, name, DynamicValue::Null)
    }

    /// Generic field setter — validates the value against the field's schema.
    pub fn set_field(&mut self, name: &str, value: DynamicValue) -> DdsResult<()> {
        self.value.set_field(&self.schema, name, value)
    }

    /// Check whether a field is set (non-null).
    pub fn is_set(&self, name: &str) -> bool {
        match self.value.field(name) {
            Some(DynamicValue::Null) => false,
            Some(_) => true,
            None => false,
        }
    }

    /// List the names of all fields in this data's schema.
    ///
    /// Only works for struct types; returns an empty list otherwise.
    pub fn field_names(&self) -> Vec<&str> {
        match &self.schema {
            DynamicTypeSchema::Struct { fields, .. } => {
                fields.iter().map(|f| f.name.as_str()).collect()
            }
            _ => Vec::new(),
        }
    }

    /// Validate the current value against the schema.
    pub fn validate(&self) -> DdsResult<()> {
        self.value.validate_against(&self.schema)
    }
}

impl std::fmt::Debug for DynamicData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DynamicData")
            .field("schema", &self.schema)
            .field("value", &self.value)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicFieldSchema {
    pub name: String,
    pub member_id: u32,
    pub value: DynamicTypeSchema,
    pub optional: bool,
    pub key: bool,
    pub external: bool,
    pub must_understand: bool,
    pub hash_id_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicUnionCaseSchema {
    pub name: String,
    pub member_id: u32,
    pub value: DynamicTypeSchema,
    pub labels: Vec<i32>,
    pub default: bool,
    pub external: bool,
    pub hash_id_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicEnumLiteralSchema {
    pub name: String,
    pub value: i32,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicBitmaskFieldSchema {
    pub name: String,
    pub position: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicTypeSchema {
    Primitive(DynamicPrimitiveKind),
    String {
        bound: Option<u32>,
    },
    Sequence {
        name: String,
        bound: Option<u32>,
        element: Box<DynamicTypeSchema>,
    },
    Array {
        name: String,
        bounds: Vec<u32>,
        element: Box<DynamicTypeSchema>,
    },
    Struct {
        name: String,
        base: Option<Box<DynamicTypeSchema>>,
        fields: Vec<DynamicFieldSchema>,
        extensibility: Option<DynamicTypeExtensibility>,
        autoid: Option<DynamicTypeAutoId>,
        nested: bool,
    },
    Enum {
        name: String,
        literals: Vec<DynamicEnumLiteralSchema>,
        bit_bound: Option<u16>,
    },
    Bitmask {
        name: String,
        fields: Vec<DynamicBitmaskFieldSchema>,
        bit_bound: Option<u16>,
    },
    Alias {
        name: String,
        target: Box<DynamicTypeSchema>,
    },
    Union {
        name: String,
        discriminator: Box<DynamicTypeSchema>,
        cases: Vec<DynamicUnionCaseSchema>,
        autoid: Option<DynamicTypeAutoId>,
        nested: bool,
    },
    Map {
        name: String,
        bound: Option<u32>,
        key: Box<DynamicTypeSchema>,
        value: Box<DynamicTypeSchema>,
    },
}

impl DynamicTypeSchema {
    pub fn default_value(&self) -> DynamicValue {
        match self {
            DynamicTypeSchema::Primitive(kind) => match kind {
                DynamicPrimitiveKind::Boolean => DynamicValue::Bool(false),
                DynamicPrimitiveKind::Byte => DynamicValue::Byte(0),
                DynamicPrimitiveKind::Int8 => DynamicValue::Int8(0),
                DynamicPrimitiveKind::UInt8 => DynamicValue::UInt8(0),
                DynamicPrimitiveKind::Int16 => DynamicValue::Int16(0),
                DynamicPrimitiveKind::UInt16 => DynamicValue::UInt16(0),
                DynamicPrimitiveKind::Int32 => DynamicValue::Int32(0),
                DynamicPrimitiveKind::UInt32 => DynamicValue::UInt32(0),
                DynamicPrimitiveKind::Int64 => DynamicValue::Int64(0),
                DynamicPrimitiveKind::UInt64 => DynamicValue::UInt64(0),
                DynamicPrimitiveKind::Float32 => DynamicValue::Float32(0.0),
                DynamicPrimitiveKind::Float64 => DynamicValue::Float64(0.0),
                DynamicPrimitiveKind::Char8 => DynamicValue::Char8(0),
                DynamicPrimitiveKind::Char16 => DynamicValue::Char16(0),
            },
            DynamicTypeSchema::String { .. } => DynamicValue::String(String::new()),
            DynamicTypeSchema::Sequence { .. } => DynamicValue::Sequence(Vec::new()),
            DynamicTypeSchema::Array {
                bounds, element, ..
            } => {
                let len = bounds
                    .iter()
                    .copied()
                    .fold(1usize, |acc, b| acc.saturating_mul(b as usize));
                DynamicValue::Array((0..len).map(|_| element.default_value()).collect())
            }
            DynamicTypeSchema::Struct { fields, base, .. } => {
                let mut values = match base.as_deref() {
                    Some(DynamicTypeSchema::Struct { .. }) => {
                        match base.as_ref().unwrap().default_value() {
                            DynamicValue::Struct(map) => map,
                            _ => BTreeMap::new(),
                        }
                    }
                    _ => BTreeMap::new(),
                };
                for field in fields {
                    values.insert(
                        field.name.clone(),
                        if field.optional {
                            DynamicValue::Null
                        } else {
                            field.value.default_value()
                        },
                    );
                }
                DynamicValue::Struct(values)
            }
            DynamicTypeSchema::Enum { literals, .. } => {
                let value = literals
                    .iter()
                    .find(|literal| literal.default)
                    .or_else(|| literals.first())
                    .map(|literal| literal.value)
                    .unwrap_or(0);
                DynamicValue::Enum { value }
            }
            DynamicTypeSchema::Bitmask { .. } => DynamicValue::Bitmask(0),
            DynamicTypeSchema::Alias { target, .. } => target.default_value(),
            DynamicTypeSchema::Union { cases, .. } => {
                if let Some(case) = cases
                    .iter()
                    .find(|case| case.default)
                    .or_else(|| cases.first())
                {
                    let discriminator = case.labels.first().copied().unwrap_or_default();
                    DynamicValue::Union {
                        discriminator,
                        field: case.name.clone(),
                        value: Box::new(case.value.default_value()),
                    }
                } else {
                    DynamicValue::Null
                }
            }
            DynamicTypeSchema::Map { .. } => DynamicValue::Map(Vec::new()),
        }
    }

    pub fn field(&self, name: &str) -> Option<&DynamicFieldSchema> {
        match self {
            DynamicTypeSchema::Struct { fields, base, .. } => fields
                .iter()
                .find(|field| field.name == name)
                .or_else(|| base.as_deref().and_then(|base| base.field(name))),
            _ => None,
        }
    }

    pub fn union_case(&self, name: &str) -> Option<&DynamicUnionCaseSchema> {
        match self {
            DynamicTypeSchema::Union { cases, .. } => cases.iter().find(|case| case.name == name),
            _ => None,
        }
    }

    /// Check whether this type schema is assignable from `other` according to
    /// DDS-XTypes assignability rules.
    ///
    /// Returns `true` if a DataWriter of type `other` can write samples that
    /// are compatible with a DataReader of type `self`.
    pub fn is_assignable_from(&self, other: &DynamicTypeSchema) -> bool {
        match (self, other) {
            // Primitives: exact match required
            (DynamicTypeSchema::Primitive(a), DynamicTypeSchema::Primitive(b)) => a == b,

            // String: self bound must be >= other bound (or unbounded)
            (DynamicTypeSchema::String { bound: a }, DynamicTypeSchema::String { bound: b }) => {
                match (a, b) {
                    (None, _) => true,
                    (Some(a_bound), Some(b_bound)) => a_bound >= b_bound,
                    (Some(_), None) => false,
                }
            }

            // Sequence: compatible bounds and element type
            (
                DynamicTypeSchema::Sequence {
                    bound: a_bound,
                    element: a_elem,
                    ..
                },
                DynamicTypeSchema::Sequence {
                    bound: b_bound,
                    element: b_elem,
                    ..
                },
            ) => {
                let bound_ok = match (a_bound, b_bound) {
                    (None, _) => true,
                    (Some(a), Some(b)) => a >= b,
                    (Some(_), None) => false,
                };
                bound_ok && a_elem.is_assignable_from(b_elem)
            }

            // Array: exact dimensions and compatible element
            (
                DynamicTypeSchema::Array {
                    bounds: a_bounds,
                    element: a_elem,
                    ..
                },
                DynamicTypeSchema::Array {
                    bounds: b_bounds,
                    element: b_elem,
                    ..
                },
            ) => a_bounds == b_bounds && a_elem.is_assignable_from(b_elem),

            // Struct: extensibility-driven rules
            (
                DynamicTypeSchema::Struct {
                    base: a_base,
                    fields: a_fields,
                    extensibility: a_ext,
                    ..
                },
                DynamicTypeSchema::Struct {
                    base: b_base,
                    fields: b_fields,
                    extensibility: b_ext,
                    ..
                },
            ) => Self::is_struct_assignable_from(
                a_base.as_deref(),
                a_fields,
                *a_ext,
                b_base.as_deref(),
                b_fields,
                *b_ext,
            ),

            // Enum: exact literal set
            (
                DynamicTypeSchema::Enum {
                    literals: a_literals,
                    bit_bound: a_bb,
                    ..
                },
                DynamicTypeSchema::Enum {
                    literals: b_literals,
                    bit_bound: b_bb,
                    ..
                },
            ) => a_literals == b_literals && a_bb == b_bb,

            // Bitmask: exact field set and bit bound
            (
                DynamicTypeSchema::Bitmask {
                    fields: a_fields,
                    bit_bound: a_bb,
                    ..
                },
                DynamicTypeSchema::Bitmask {
                    fields: b_fields,
                    bit_bound: b_bb,
                    ..
                },
            ) => a_fields == b_fields && a_bb == b_bb,

            // Union: compatible discriminator and cases
            (
                DynamicTypeSchema::Union {
                    discriminator: a_disc,
                    cases: a_cases,
                    ..
                },
                DynamicTypeSchema::Union {
                    discriminator: b_disc,
                    cases: b_cases,
                    ..
                },
            ) => {
                a_disc.is_assignable_from(b_disc)
                    && a_cases.len() == b_cases.len()
                    && a_cases.iter().zip(b_cases.iter()).all(|(a, b)| {
                        a.name == b.name
                            && a.labels == b.labels
                            && a.default == b.default
                            && a.value.is_assignable_from(&b.value)
                    })
            }

            // Map: compatible bounds, key and value
            (
                DynamicTypeSchema::Map {
                    bound: a_bound,
                    key: a_key,
                    value: a_val,
                    ..
                },
                DynamicTypeSchema::Map {
                    bound: b_bound,
                    key: b_key,
                    value: b_val,
                    ..
                },
            ) => {
                let bound_ok = match (a_bound, b_bound) {
                    (None, _) => true,
                    (Some(a), Some(b)) => a >= b,
                    (Some(_), None) => false,
                };
                bound_ok && a_key.is_assignable_from(b_key) && a_val.is_assignable_from(b_val)
            }

            // Alias: resolve and compare
            (DynamicTypeSchema::Alias { target, .. }, _) => target.is_assignable_from(other),
            (_, DynamicTypeSchema::Alias { target, .. }) => self.is_assignable_from(target),

            // Default: incompatible
            _ => false,
        }
    }

    fn is_struct_assignable_from(
        a_base: Option<&DynamicTypeSchema>,
        a_fields: &[DynamicFieldSchema],
        a_ext: Option<DynamicTypeExtensibility>,
        b_base: Option<&DynamicTypeSchema>,
        b_fields: &[DynamicFieldSchema],
        b_ext: Option<DynamicTypeExtensibility>,
    ) -> bool {
        use DynamicTypeExtensibility::*;

        let a_ext = a_ext.unwrap_or(Final);
        let b_ext = b_ext.unwrap_or(Final);

        // Final: exact match required (same fields, same order)
        if a_ext == Final {
            return b_ext == Final
                && a_base == b_base
                && a_fields.len() == b_fields.len()
                && a_fields.iter().zip(b_fields.iter()).all(|(a, b)| {
                    a.name == b.name
                        && a.value.is_assignable_from(&b.value)
                        && a.optional == b.optional
                        && a.key == b.key
                });
        }

        // Appendable: other's fields must be a prefix of self's fields
        if a_ext == Appendable {
            // Bases must be assignable if present
            if let (Some(a_b), Some(b_b)) = (a_base, b_base) {
                if !a_b.is_assignable_from(b_b) {
                    return false;
                }
            } else if a_base.is_some() || b_base.is_some() {
                return false;
            }

            // Other's fields must match self's fields up to other's length
            if b_fields.len() > a_fields.len() {
                return false;
            }
            for (a_field, b_field) in a_fields.iter().zip(b_fields.iter()) {
                if a_field.name != b_field.name
                    || !a_field.value.is_assignable_from(&b_field.value)
                    || a_field.optional != b_field.optional
                    || a_field.key != b_field.key
                {
                    return false;
                }
            }
            return true;
        }

        // Mutable: relaxed matching by field name
        if a_ext == Mutable {
            // Bases must be assignable if present
            if let (Some(a_b), Some(b_b)) = (a_base, b_base) {
                if !a_b.is_assignable_from(b_b) {
                    return false;
                }
            } else if a_base.is_some() || b_base.is_some() {
                return false;
            }

            // All fields in other must exist in self with compatible type
            for b_field in b_fields {
                let Some(a_field) = a_fields.iter().find(|f| f.name == b_field.name) else {
                    return false;
                };
                if !a_field.value.is_assignable_from(&b_field.value) {
                    return false;
                }
            }

            // All non-optional fields in self must exist in other
            for a_field in a_fields {
                if !a_field.optional && !b_fields.iter().any(|f| f.name == a_field.name) {
                    return false;
                }
            }

            return true;
        }

        false
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DynamicValue {
    Bool(bool),
    Byte(u8),
    Int8(i8),
    UInt8(u8),
    Int16(i16),
    UInt16(u16),
    Int32(i32),
    UInt32(u32),
    Int64(i64),
    UInt64(u64),
    Float32(f32),
    Float64(f64),
    Char8(u8),
    Char16(u16),
    String(String),
    Sequence(Vec<DynamicValue>),
    Array(Vec<DynamicValue>),
    Struct(BTreeMap<String, DynamicValue>),
    Enum {
        value: i32,
    },
    Bitmask(u64),
    Union {
        discriminator: i32,
        field: String,
        value: Box<DynamicValue>,
    },
    Map(Vec<(DynamicValue, DynamicValue)>),
    Null,
}

impl DynamicValue {
    pub fn new(schema: &DynamicTypeSchema) -> Self {
        schema.default_value()
    }

    pub fn as_struct(&self) -> Option<&BTreeMap<String, DynamicValue>> {
        match self {
            DynamicValue::Struct(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn as_struct_mut(&mut self) -> Option<&mut BTreeMap<String, DynamicValue>> {
        match self {
            DynamicValue::Struct(fields) => Some(fields),
            _ => None,
        }
    }

    pub fn field(&self, name: &str) -> Option<&DynamicValue> {
        self.as_struct()?.get(name)
    }

    pub fn set_field(
        &mut self,
        schema: &DynamicTypeSchema,
        name: &str,
        value: DynamicValue,
    ) -> DdsResult<()> {
        let field = schema.field(name).ok_or_else(|| {
            DdsError::BadParameter(format!("field '{}' not present in dynamic schema", name))
        })?;
        value.validate_against(&field.value)?;
        let fields = self
            .as_struct_mut()
            .ok_or_else(|| DdsError::BadParameter("dynamic value is not a struct".into()))?;
        fields.insert(name.to_string(), value);
        Ok(())
    }

    pub fn as_sequence(&self) -> Option<&Vec<DynamicValue>> {
        match self {
            DynamicValue::Sequence(values) | DynamicValue::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_sequence_mut(&mut self) -> Option<&mut Vec<DynamicValue>> {
        match self {
            DynamicValue::Sequence(values) | DynamicValue::Array(values) => Some(values),
            _ => None,
        }
    }

    pub fn push(&mut self, schema: &DynamicTypeSchema, value: DynamicValue) -> DdsResult<()> {
        let (bound, element_schema) = match schema {
            DynamicTypeSchema::Sequence { bound, element, .. } => (*bound, element.as_ref()),
            DynamicTypeSchema::Map { .. } | DynamicTypeSchema::Array { .. } => {
                return Err(DdsError::BadParameter(
                    "push is only supported for dynamic sequences".into(),
                ))
            }
            _ => {
                return Err(DdsError::BadParameter(
                    "dynamic value is not backed by a sequence schema".into(),
                ))
            }
        };
        value.validate_against(element_schema)?;
        let values = self
            .as_sequence_mut()
            .ok_or_else(|| DdsError::BadParameter("dynamic value is not a sequence".into()))?;
        if let Some(bound) = bound {
            if values.len() >= bound as usize {
                return Err(DdsError::BadParameter(format!(
                    "sequence already at bound {}",
                    bound
                )));
            }
        }
        values.push(value);
        Ok(())
    }

    pub fn union(
        schema: &DynamicTypeSchema,
        discriminator: i32,
        field: impl Into<String>,
        value: DynamicValue,
    ) -> DdsResult<Self> {
        let field = field.into();
        let case = schema.union_case(&field).ok_or_else(|| {
            DdsError::BadParameter(format!("union field '{}' not present in schema", field))
        })?;
        value.validate_against(&case.value)?;
        let union_value = DynamicValue::Union {
            discriminator,
            field,
            value: Box::new(value),
        };
        union_value.validate_against(schema)?;
        Ok(union_value)
    }

    pub fn union_discriminator(&self) -> Option<i32> {
        match self {
            DynamicValue::Union { discriminator, .. } => Some(*discriminator),
            _ => None,
        }
    }

    pub fn union_field_name(&self) -> Option<&str> {
        match self {
            DynamicValue::Union { field, .. } => Some(field.as_str()),
            _ => None,
        }
    }

    pub fn union_field_value(&self) -> Option<&DynamicValue> {
        match self {
            DynamicValue::Union { value, .. } => Some(value.as_ref()),
            _ => None,
        }
    }

    pub fn validate_against(&self, schema: &DynamicTypeSchema) -> DdsResult<()> {
        match (self, schema) {
            (
                DynamicValue::Bool(_),
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Boolean),
            ) => Ok(()),
            (DynamicValue::Byte(_), DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Byte)) => {
                Ok(())
            }
            (DynamicValue::Int8(_), DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int8)) => {
                Ok(())
            }
            (DynamicValue::UInt8(_), DynamicTypeSchema::Primitive(DynamicPrimitiveKind::UInt8)) => {
                Ok(())
            }
            (DynamicValue::Int16(_), DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int16)) => {
                Ok(())
            }
            (
                DynamicValue::UInt16(_),
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::UInt16),
            ) => Ok(()),
            (DynamicValue::Int32(_), DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32)) => {
                Ok(())
            }
            (
                DynamicValue::UInt32(_),
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::UInt32),
            ) => Ok(()),
            (DynamicValue::Int64(_), DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int64)) => {
                Ok(())
            }
            (
                DynamicValue::UInt64(_),
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::UInt64),
            ) => Ok(()),
            (
                DynamicValue::Float32(_),
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Float32),
            ) => Ok(()),
            (
                DynamicValue::Float64(_),
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Float64),
            ) => Ok(()),
            (DynamicValue::Char8(_), DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Char8)) => {
                Ok(())
            }
            (
                DynamicValue::Char16(_),
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Char16),
            ) => Ok(()),
            (DynamicValue::String(value), DynamicTypeSchema::String { bound }) => {
                if let Some(bound) = bound {
                    if value.len() > *bound as usize {
                        return Err(DdsError::BadParameter(format!(
                            "string length {} exceeds bound {}",
                            value.len(),
                            bound
                        )));
                    }
                }
                Ok(())
            }
            (
                DynamicValue::Sequence(values),
                DynamicTypeSchema::Sequence { bound, element, .. },
            ) => {
                if let Some(bound) = bound {
                    if values.len() > *bound as usize {
                        return Err(DdsError::BadParameter(format!(
                            "sequence length {} exceeds bound {}",
                            values.len(),
                            bound
                        )));
                    }
                }
                for value in values {
                    value.validate_against(element)?;
                }
                Ok(())
            }
            (
                DynamicValue::Array(values),
                DynamicTypeSchema::Array {
                    bounds, element, ..
                },
            ) => {
                let expected_len = bounds
                    .iter()
                    .copied()
                    .fold(1usize, |acc, b| acc.saturating_mul(b as usize));
                if values.len() != expected_len {
                    return Err(DdsError::BadParameter(format!(
                        "array value length {} does not match expected flattened length {}",
                        values.len(),
                        expected_len
                    )));
                }
                for value in values {
                    value.validate_against(element)?;
                }
                Ok(())
            }
            (DynamicValue::Struct(values), DynamicTypeSchema::Struct { fields, base, .. }) => {
                if let Some(base) = base {
                    self.validate_against(base)?;
                }
                for field in fields {
                    match values.get(&field.name) {
                        Some(value) => {
                            if field.optional && matches!(value, DynamicValue::Null) {
                                continue;
                            }
                            value.validate_against(&field.value)?;
                        }
                        None if field.optional => {}
                        None => {
                            return Err(DdsError::BadParameter(format!(
                                "missing required dynamic field '{}'",
                                field.name
                            )))
                        }
                    }
                }
                Ok(())
            }
            (
                DynamicValue::Enum { value },
                DynamicTypeSchema::Enum {
                    literals,
                    bit_bound,
                    ..
                },
            ) => {
                if !literals.iter().any(|literal| literal.value == *value) {
                    return Err(DdsError::BadParameter(format!(
                        "enum value {} not defined in schema",
                        value
                    )));
                }
                if let Some(bit_bound) = bit_bound {
                    if *bit_bound < 32 {
                        let max = if *bit_bound == 31 {
                            i32::MAX as i64
                        } else {
                            (1i64 << *bit_bound) - 1
                        };
                        if (*value as i64) > max {
                            return Err(DdsError::BadParameter(format!(
                                "enum value {} exceeds bit bound {}",
                                value, bit_bound
                            )));
                        }
                    }
                }
                Ok(())
            }
            (DynamicValue::Bitmask(value), DynamicTypeSchema::Bitmask { bit_bound, .. }) => {
                if let Some(bit_bound) = bit_bound {
                    if *bit_bound < 64 && *value >= (1u64 << *bit_bound) {
                        return Err(DdsError::BadParameter(format!(
                            "bitmask value {} exceeds bit bound {}",
                            value, bit_bound
                        )));
                    }
                }
                Ok(())
            }
            (
                DynamicValue::Union {
                    discriminator,
                    field,
                    value,
                },
                DynamicTypeSchema::Union { cases, .. },
            ) => {
                let case = cases
                    .iter()
                    .find(|case| case.name == *field)
                    .ok_or_else(|| {
                        DdsError::BadParameter(format!(
                            "union field '{}' not defined in schema",
                            field
                        ))
                    })?;
                let label_match = case.default || case.labels.contains(discriminator);
                if !label_match {
                    return Err(DdsError::BadParameter(format!(
                        "union discriminator {} does not match field '{}'",
                        discriminator, field
                    )));
                }
                value.validate_against(&case.value)
            }
            (
                DynamicValue::Map(entries),
                DynamicTypeSchema::Map {
                    bound, key, value, ..
                },
            ) => {
                if let Some(bound) = bound {
                    if entries.len() > *bound as usize {
                        return Err(DdsError::BadParameter(format!(
                            "map length {} exceeds bound {}",
                            entries.len(),
                            bound
                        )));
                    }
                }
                for (k, v) in entries {
                    k.validate_against(key)?;
                    v.validate_against(value)?;
                }
                Ok(())
            }
            (value, DynamicTypeSchema::Alias { target, .. }) => value.validate_against(target),
            (DynamicValue::Null, _) => Ok(()),
            (value, schema) => Err(DdsError::BadParameter(format!(
                "dynamic value {value:?} does not match schema {schema:?}"
            ))),
        }
    }
}
