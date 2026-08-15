use cyclonedds::{
    DomainParticipant, DynamicEnumLiteralValue, DynamicPrimitiveKind, DynamicTypeBuilder,
    DynamicTypeExtensibility, DynamicTypeSpec,
};

#[test]
fn builder_add_field_creates_struct() {
    let participant = DomainParticipant::new(0).unwrap();
    let dynamic_type = DynamicTypeBuilder::structure("Point")
        .appendable()
        .add_field("x", DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32))
        .add_field("y", DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32))
        .build(&participant)
        .unwrap();

    let schema = dynamic_type.schema();
    match schema {
        cyclonedds::DynamicTypeSchema::Struct {
            name,
            fields,
            extensibility,
            ..
        } => {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
            assert_eq!(*extensibility, Some(DynamicTypeExtensibility::Appendable));
        }
        _ => panic!("expected struct schema"),
    }
}

#[test]
fn builder_add_enum_variant_creates_enum() {
    let participant = DomainParticipant::new(0).unwrap();
    let dynamic_type = DynamicTypeBuilder::enumeration("Color")
        .add_enum_variant("Red", DynamicEnumLiteralValue::Explicit(0), false)
        .add_enum_variant("Green", DynamicEnumLiteralValue::Explicit(1), true)
        .build(&participant)
        .unwrap();

    let schema = dynamic_type.schema();
    match schema {
        cyclonedds::DynamicTypeSchema::Enum { name, literals, .. } => {
            assert_eq!(name, "Color");
            assert_eq!(literals.len(), 2);
            assert_eq!(literals[0].name, "Red");
            assert_eq!(literals[0].value, 0);
            assert_eq!(literals[1].name, "Green");
            assert_eq!(literals[1].value, 1);
            assert!(literals[1].default);
        }
        _ => panic!("expected enum schema"),
    }
}

#[test]
fn builder_add_bitmask_field_creates_bitmask() {
    let participant = DomainParticipant::new(0).unwrap();
    let dynamic_type = DynamicTypeBuilder::bitmask("Flags")
        .add_bitmask_field("FlagA", Some(0))
        .add_bitmask_field("FlagB", Some(1))
        .build(&participant)
        .unwrap();

    let schema = dynamic_type.schema();
    match schema {
        cyclonedds::DynamicTypeSchema::Bitmask { name, fields, .. } => {
            assert_eq!(name, "Flags");
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "FlagA");
            assert_eq!(fields[0].position, 0);
            assert_eq!(fields[1].name, "FlagB");
            assert_eq!(fields[1].position, 1);
        }
        _ => panic!("expected bitmask schema"),
    }
}

#[test]
fn builder_add_union_case_creates_union() {
    let participant = DomainParticipant::new(0).unwrap();
    let dynamic_type = DynamicTypeBuilder::union(
        "MyUnion",
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
    )
    .add_union_case(
        "case_a",
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
        &[1],
    )
    .add_union_case(
        "case_b",
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
        &[2],
    )
    .build(&participant)
    .unwrap();

    let schema = dynamic_type.schema();
    match schema {
        cyclonedds::DynamicTypeSchema::Union { name, cases, .. } => {
            assert_eq!(name, "MyUnion");
            assert_eq!(cases.len(), 2);
            assert_eq!(cases[0].name, "case_a");
            assert_eq!(cases[0].labels, vec![1]);
            assert_eq!(cases[1].name, "case_b");
            assert_eq!(cases[1].labels, vec![2]);
        }
        _ => panic!("expected union schema"),
    }
}

/// Every public constructor produces a schema, so `to_schema`'s error arms are
/// unreachable from outside the crate.
///
/// `to_schema` used to `expect`/`panic!` on a missing sub-type. It now returns
/// `DdsResult`, but that is hardening rather than a fix and this test is what
/// says so: no caller can reach those arms. `DynamicTypeBuilder::new` is
/// private, each constructor whose kind needs a sub-type takes it as an
/// argument, and the setters take values rather than `Option`s — so a sub-type
/// can be replaced but never cleared. If a future constructor breaks that, this
/// fails here rather than unwinding in a user's thread.
#[test]
fn every_public_constructor_builds_without_panicking() {
    let participant = DomainParticipant::new(0).unwrap();
    let i32_spec = || DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32);

    let builders: Vec<(&str, DynamicTypeBuilder)> = vec![
        ("structure", DynamicTypeBuilder::structure("S")),
        ("enumeration", DynamicTypeBuilder::enumeration("E")),
        ("bitmask", DynamicTypeBuilder::bitmask("B")),
        ("alias", DynamicTypeBuilder::alias("A", i32_spec())),
        ("string8", DynamicTypeBuilder::string8(None)),
        ("bounded_string8", DynamicTypeBuilder::bounded_string8(32)),
        ("unbounded_string8", DynamicTypeBuilder::unbounded_string8()),
        (
            "sequence",
            DynamicTypeBuilder::sequence("Sq", i32_spec(), None),
        ),
        (
            "bounded_sequence",
            DynamicTypeBuilder::bounded_sequence("BSq", i32_spec(), 4),
        ),
        (
            "unbounded_sequence",
            DynamicTypeBuilder::unbounded_sequence("USq", i32_spec()),
        ),
        (
            "array",
            DynamicTypeBuilder::array("Ar", i32_spec(), vec![3]),
        ),
        ("union", DynamicTypeBuilder::union("U", i32_spec())),
    ];

    for (what, builder) in builders {
        let built = builder.build(&participant);
        assert!(
            built.is_ok(),
            "DynamicTypeBuilder::{what} failed to build: {:?}",
            built.err()
        );
    }

    // `map`/`bounded_map` are excluded above because they cannot succeed:
    // CycloneDDS 11.0.1 returns DDS_RETCODE_UNSUPPORTED for DDS_DYNAMIC_MAP
    // (`dds_dynamic_type.c:237`). They still reach `to_schema` without panicking,
    // which is what this test is about; the error is asserted in
    // `error_retcode_mapping.rs`.
    for (what, builder) in [
        (
            "map",
            DynamicTypeBuilder::map("M", i32_spec(), i32_spec(), None),
        ),
        (
            "bounded_map",
            DynamicTypeBuilder::bounded_map("BM", i32_spec(), i32_spec(), 8),
        ),
    ] {
        let err = builder
            .build(&participant)
            .expect_err("CycloneDDS 11 does not implement dynamic maps");
        assert!(
            matches!(err, cyclonedds::DdsError::Unsupported(_)),
            "DynamicTypeBuilder::{what} should report Unsupported, got {err:?}"
        );
    }
}
