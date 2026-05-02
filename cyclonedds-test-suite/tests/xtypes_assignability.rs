use cyclonedds::{
    DynamicFieldSchema, DynamicPrimitiveKind, DynamicTypeExtensibility, DynamicTypeSchema,
};

fn make_struct(
    name: &str,
    ext: Option<DynamicTypeExtensibility>,
    fields: Vec<(&str, DynamicTypeSchema, bool)>,
) -> DynamicTypeSchema {
    DynamicTypeSchema::Struct {
        name: name.to_string(),
        base: None,
        fields: fields
            .into_iter()
            .map(|(n, t, opt)| DynamicFieldSchema {
                name: n.to_string(),
                member_id: 0,
                value: t,
                optional: opt,
                key: false,
                external: false,
                must_understand: false,
                hash_id_name: None,
            })
            .collect(),
        extensibility: ext,
        autoid: None,
        nested: false,
    }
}

#[test]
fn identical_types_are_assignable() {
    let s = make_struct(
        "Point",
        Some(DynamicTypeExtensibility::Final),
        vec![
            (
                "x",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
            (
                "y",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
        ],
    );
    assert!(s.is_assignable_from(&s));
}

#[test]
fn final_struct_requires_exact_match() {
    let a = make_struct(
        "A",
        Some(DynamicTypeExtensibility::Final),
        vec![
            (
                "x",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
            (
                "y",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
        ],
    );
    let b = make_struct(
        "B",
        Some(DynamicTypeExtensibility::Final),
        vec![
            (
                "x",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
            (
                "y",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int64),
                false,
            ),
        ],
    );
    assert!(!a.is_assignable_from(&b));
}

#[test]
fn appendable_allows_extra_fields() {
    let base = make_struct(
        "Base",
        Some(DynamicTypeExtensibility::Appendable),
        vec![(
            "x",
            DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
            false,
        )],
    );
    let extended = make_struct(
        "Extended",
        Some(DynamicTypeExtensibility::Appendable),
        vec![
            (
                "x",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
            (
                "y",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
        ],
    );
    // Extended can receive Base (Base fields are prefix of Extended)
    assert!(extended.is_assignable_from(&base));
    // Base cannot receive Extended (Extended has more fields)
    assert!(!base.is_assignable_from(&extended));
}

#[test]
fn appendable_final_to_appendable() {
    let fin = make_struct(
        "FinalStruct",
        Some(DynamicTypeExtensibility::Final),
        vec![(
            "x",
            DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
            false,
        )],
    );
    let app = make_struct(
        "AppStruct",
        Some(DynamicTypeExtensibility::Appendable),
        vec![
            (
                "x",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
            (
                "y",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
        ],
    );
    // Appendable can receive Final with matching prefix
    assert!(app.is_assignable_from(&fin));
}

#[test]
fn mutable_allows_reordering_and_optional() {
    let a = make_struct(
        "A",
        Some(DynamicTypeExtensibility::Mutable),
        vec![
            (
                "x",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
            (
                "y",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                true,
            ),
        ],
    );
    let b = make_struct(
        "B",
        Some(DynamicTypeExtensibility::Mutable),
        vec![
            (
                "y",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                true,
            ),
            (
                "x",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
        ],
    );
    assert!(a.is_assignable_from(&b));
    assert!(b.is_assignable_from(&a));
}

#[test]
fn mutable_requires_non_optional_in_self() {
    let a = make_struct(
        "A",
        Some(DynamicTypeExtensibility::Mutable),
        vec![
            (
                "x",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
            (
                "y",
                DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
                false,
            ),
        ],
    );
    let b = make_struct(
        "B",
        Some(DynamicTypeExtensibility::Mutable),
        vec![(
            "x",
            DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32),
            false,
        )],
    );
    // A requires y, but B doesn't have it -> A cannot receive B
    assert!(!a.is_assignable_from(&b));
    // B only has x, but A writes x and y -> B cannot receive A (y is extra)
    assert!(!b.is_assignable_from(&a));
}

#[test]
fn primitive_mismatch_not_assignable() {
    let i32 = DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32);
    let i64 = DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int64);
    assert!(!i32.is_assignable_from(&i64));
    assert!(!i64.is_assignable_from(&i32));
}

#[test]
fn string_bound_compatibility() {
    let unbounded = DynamicTypeSchema::String { bound: None };
    let bounded_10 = DynamicTypeSchema::String { bound: Some(10) };
    let bounded_5 = DynamicTypeSchema::String { bound: Some(5) };

    // Unbounded accepts any
    assert!(unbounded.is_assignable_from(&bounded_10));
    assert!(unbounded.is_assignable_from(&bounded_5));

    // Bounded must be >= other
    assert!(bounded_10.is_assignable_from(&bounded_5));
    assert!(!bounded_5.is_assignable_from(&bounded_10));
}

#[test]
fn array_exact_match_required() {
    let a = DynamicTypeSchema::Array {
        name: "A".to_string(),
        bounds: vec![2, 3],
        element: Box::new(DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32)),
    };
    let b = DynamicTypeSchema::Array {
        name: "B".to_string(),
        bounds: vec![2, 3],
        element: Box::new(DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32)),
    };
    let c = DynamicTypeSchema::Array {
        name: "C".to_string(),
        bounds: vec![3, 2],
        element: Box::new(DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32)),
    };
    assert!(a.is_assignable_from(&b));
    assert!(!a.is_assignable_from(&c));
}

#[test]
fn sequence_bound_compatibility() {
    let unbounded = DynamicTypeSchema::Sequence {
        name: "S".to_string(),
        bound: None,
        element: Box::new(DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32)),
    };
    let bounded_10 = DynamicTypeSchema::Sequence {
        name: "S".to_string(),
        bound: Some(10),
        element: Box::new(DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32)),
    };
    let bounded_5 = DynamicTypeSchema::Sequence {
        name: "S".to_string(),
        bound: Some(5),
        element: Box::new(DynamicTypeSchema::Primitive(DynamicPrimitiveKind::Int32)),
    };

    assert!(unbounded.is_assignable_from(&bounded_10));
    assert!(bounded_10.is_assignable_from(&bounded_5));
    assert!(!bounded_5.is_assignable_from(&bounded_10));
}

#[test]
fn enum_exact_match() {
    let a = DynamicTypeSchema::Enum {
        name: "Color".to_string(),
        literals: vec![
            cyclonedds::DynamicEnumLiteralSchema {
                name: "Red".to_string(),
                value: 0,
                default: true,
            },
            cyclonedds::DynamicEnumLiteralSchema {
                name: "Green".to_string(),
                value: 1,
                default: false,
            },
        ],
        bit_bound: Some(8),
    };
    let b = DynamicTypeSchema::Enum {
        name: "Color".to_string(),
        literals: vec![
            cyclonedds::DynamicEnumLiteralSchema {
                name: "Red".to_string(),
                value: 0,
                default: true,
            },
            cyclonedds::DynamicEnumLiteralSchema {
                name: "Green".to_string(),
                value: 1,
                default: false,
            },
        ],
        bit_bound: Some(8),
    };
    let c = DynamicTypeSchema::Enum {
        name: "Color".to_string(),
        literals: vec![
            cyclonedds::DynamicEnumLiteralSchema {
                name: "Red".to_string(),
                value: 0,
                default: true,
            },
            cyclonedds::DynamicEnumLiteralSchema {
                name: "Blue".to_string(),
                value: 2,
                default: false,
            },
        ],
        bit_bound: Some(8),
    };
    assert!(a.is_assignable_from(&b));
    assert!(!a.is_assignable_from(&c));
}
