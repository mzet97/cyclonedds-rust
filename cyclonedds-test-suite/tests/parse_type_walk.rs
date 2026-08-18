//! `TopicDescriptor::parse_type` must walk the ops array in step.
//!
//! `adr_step` carried a hand-maintained table of ADR instruction widths and it
//! omitted `ENU`, `ARR`, `UNI` and `EXT` entirely — all fell through to 2 words.
//! `ARR` is 3 words minimum and `ENU`/`EXT` are 3, so a single array, enum or
//! nested-struct field put the walk permanently out of phase: later
//! instructions were read at the wrong offset, data words got interpreted as
//! opcodes, and phantom members appeared.
//!
//! The loop is bounds-checked, so this was never memory-unsafe — it just made a
//! public introspection API describe a type that does not exist.
//!
//! Same root cause as the derive's `ops()` scanner, in a second location. There
//! the fix was to delete the scanner, since the derive knows where it emitted
//! each instruction; here the ops array comes from CycloneDDS, so the table has
//! to be correct instead.
//!
//! The types are built through `DynamicTypeBuilder` because that is the path
//! that yields a real `TopicDescriptor` — derive-generated types carry no XTypes
//! metadata blobs, so `create_topic_descriptor` rejects them.

use cyclonedds::*;
use cyclonedds_test_suite::unique_topic;

fn descriptor(
    participant: &DomainParticipant,
    build: impl FnOnce(&mut DynamicType),
) -> TopicDescriptor {
    let mut ty = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(unique_topic("parse_walk_ty")))
        .unwrap();
    ty.add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(1))
        .unwrap();
    ty.set_member_key(1, true).unwrap();
    build(&mut ty);
    ty.register_topic_descriptor(participant, FindScope::LocalDomain, 0)
        .unwrap()
}

/// Enum member: `ADR|ENU` is 3 words, the old table said 2.
#[test]
fn enum_member_does_not_desynchronise_the_walk() {
    let participant = DomainParticipant::new(0).unwrap();
    let mut enum_ty = participant
        .create_dynamic_type(DynamicTypeBuilder::enumeration(unique_topic("pw_enum")))
        .unwrap();
    enum_ty
        .add_enum_literal("Low", DynamicEnumLiteralValue::NextAvailable, false)
        .unwrap();
    enum_ty
        .add_enum_literal("High", DynamicEnumLiteralValue::NextAvailable, true)
        .unwrap();
    let spec = enum_ty.as_spec();

    let desc = descriptor(&participant, |ty| {
        ty.add_member(DynamicMemberBuilder::new("level", spec).id(2))
            .unwrap();
        ty.add_member(DynamicMemberBuilder::primitive("tail", DynamicPrimitiveKind::Int32).id(3))
            .unwrap();
    });

    let parsed = desc.parse_type().expect("parse_type failed");
    assert_eq!(
        parsed.members.len(),
        3,
        "enum field desynchronised the walk: {:?}",
        parsed.members.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
}

/// Array member: `ADR|ARR` is 3 words minimum, the old table said 2.
#[test]
fn array_member_does_not_desynchronise_the_walk() {
    let participant = DomainParticipant::new(0).unwrap();
    let arr = participant
        .create_dynamic_type(DynamicTypeBuilder::array(
            unique_topic("pw_arr"),
            DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
            vec![8],
        ))
        .unwrap();
    let spec = arr.as_spec();

    let desc = descriptor(&participant, |ty| {
        ty.add_member(DynamicMemberBuilder::new("matrix", spec).id(2))
            .unwrap();
        ty.add_member(DynamicMemberBuilder::primitive("tail", DynamicPrimitiveKind::Int32).id(3))
            .unwrap();
    });

    let parsed = desc.parse_type().expect("parse_type failed");
    assert_eq!(
        parsed.members.len(),
        3,
        "array field desynchronised the walk: {:?}",
        parsed.members.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
}

/// Nested struct: `ADR|EXT` is 3 words (4 with the external/optional flag), the
/// old table said 2.
#[test]
fn nested_struct_does_not_desynchronise_the_walk() {
    let participant = DomainParticipant::new(0).unwrap();
    let mut inner = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(unique_topic("pw_inner")))
        .unwrap();
    inner.set_nested(true).unwrap();
    inner
        .add_member(DynamicMemberBuilder::primitive("a", DynamicPrimitiveKind::Int32).id(1))
        .unwrap();
    let spec = inner.as_spec();

    let desc = descriptor(&participant, |ty| {
        ty.add_member(DynamicMemberBuilder::new("inner", spec).id(2))
            .unwrap();
        ty.add_member(DynamicMemberBuilder::primitive("tail", DynamicPrimitiveKind::Int32).id(3))
            .unwrap();
    });

    let parsed = desc.parse_type().expect("parse_type failed");
    assert_eq!(
        parsed.members.len(),
        3,
        "nested struct desynchronised the walk: {:?}",
        parsed.members.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    assert!(parsed.has_nested_types);
}

/// Guards against over-correcting: a type made only of widths the old table got
/// right must still parse to exactly its declared members.
#[test]
fn plain_type_is_unaffected() {
    let participant = DomainParticipant::new(0).unwrap();
    let desc = descriptor(&participant, |ty| {
        ty.add_member(
            DynamicMemberBuilder::primitive("value", DynamicPrimitiveKind::Float64).id(2),
        )
        .unwrap();
        ty.add_member(DynamicMemberBuilder::primitive("count", DynamicPrimitiveKind::Int32).id(3))
            .unwrap();
    });

    let parsed = desc.parse_type().expect("parse_type failed");
    assert_eq!(parsed.members.len(), 3, "got {:?}", parsed.members);
    assert_eq!(parsed.key_count, 1);
}
