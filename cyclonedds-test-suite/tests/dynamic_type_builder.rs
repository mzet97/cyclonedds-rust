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
