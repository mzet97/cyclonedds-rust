//! `DynamicData` must survive a CDR round-trip when the type contains a field
//! whose instruction is wider than two words.
//!
//! `type_discovery.rs` held three copies of the ADR instruction-width table —
//! the schema builder, `write_value_to_native` and `read_value_from_native` —
//! all missing `ENU`, `ARR`, `UNI` and `EXT`. The writer is the dangerous one:
//! it takes `ops[i + 1]` as a byte offset and does `base.add(offset)` before
//! writing the field, so a desynchronised walk turns a `max`, an `alen` or a
//! jump word into an offset and writes out of bounds.
//!
//! An enum member is the cheapest trigger: `ADR|ENU` is 3 words, the old table
//! said 2. Everything declared after it was then read and written at the wrong
//! offsets.
//!
//! A second defect in the same function: field names were derived from the word
//! position (`(i - ops_start) / 2`) rather than an ordinal, so the same 3-word
//! field also skewed every later name and those values were silently dropped.
//!
//! Run under AddressSanitizer for the out-of-bounds write specifically:
//! ```bash
//! RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p cyclonedds-test-suite \
//!     --target x86_64-unknown-linux-gnu --test dynamic_cdr_roundtrip
//! ```

use cyclonedds::*;
use cyclonedds_test_suite::unique_topic;

/// Build a dynamic struct, register it, and recover the schema/descriptor pair
/// that `dynamic_data_to_cdr` and `cdr_to_dynamic_data` operate on.
fn discovered(
    participant: &DomainParticipant,
    build: impl FnOnce(&DomainParticipant, &mut DynamicType),
) -> DiscoveredType {
    let type_name = unique_topic("dyn_rt_type");
    let mut ty = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(type_name.clone()))
        .unwrap();
    build(participant, &mut ty);
    let type_info = ty.register().unwrap();
    discover_type_from_type_info(participant, &type_info, &type_name, 0)
        .expect("type discovery failed")
}

/// A field after an enum must keep its value across the round-trip. With the
/// old 2-word assumption the walk stepped into the middle of the enum's
/// instruction and everything after it landed at the wrong offset.
#[test]
fn field_after_an_enum_survives_the_round_trip() {
    let participant = DomainParticipant::new(0).unwrap();

    let discovered = discovered(&participant, |p, ty| {
        let mut e = p
            .create_dynamic_type(DynamicTypeBuilder::enumeration(unique_topic("dyn_rt_enum")))
            .unwrap();
        e.add_enum_literal("Low", DynamicEnumLiteralValue::NextAvailable, false)
            .unwrap();
        e.add_enum_literal("High", DynamicEnumLiteralValue::NextAvailable, true)
            .unwrap();

        ty.add_member(DynamicMemberBuilder::primitive("lead", DynamicPrimitiveKind::Int32).id(1))
            .unwrap();
        ty.add_member(DynamicMemberBuilder::new("level", e.as_spec()).id(2))
            .unwrap();
        // The member that the desynchronised walk used to corrupt.
        ty.add_member(DynamicMemberBuilder::primitive("trail", DynamicPrimitiveKind::Int32).id(3))
            .unwrap();
    });

    // Descriptor-derived schemas name members field_0, field_1, ... which is the
    // naming the writer and reader agree on.
    let mut data = DynamicData::new(&discovered.type_schema);
    let names = data.field_names();
    assert!(
        names.len() >= 3,
        "schema lost members while walking the ops: {names:?}"
    );
    data.set_i32("field_0", 1234).unwrap();
    data.set_i32("field_2", -99).unwrap();

    let bytes = dynamic_data_to_cdr(&data, &discovered.topic_descriptor)
        .expect("dynamic_data_to_cdr failed");
    assert!(bytes.len() > 4, "implausible CDR length {}", bytes.len());

    let back = cdr_to_dynamic_data(
        &bytes,
        &discovered.type_schema,
        &discovered.topic_descriptor,
    )
    .expect("cdr_to_dynamic_data failed");

    assert_eq!(back.get_i32("field_0").unwrap(), 1234, "leading field lost");
    assert_eq!(
        back.get_i32("field_2").unwrap(),
        -99,
        "field after the enum was written or read at the wrong offset"
    );
}

/// Same shape without the wide field: guards against over-correcting the table.
#[test]
fn plain_struct_round_trips() {
    let participant = DomainParticipant::new(0).unwrap();

    let discovered = discovered(&participant, |_, ty| {
        ty.add_member(DynamicMemberBuilder::primitive("a", DynamicPrimitiveKind::Int32).id(1))
            .unwrap();
        ty.add_member(DynamicMemberBuilder::primitive("b", DynamicPrimitiveKind::Int32).id(2))
            .unwrap();
    });

    let mut data = DynamicData::new(&discovered.type_schema);
    data.set_i32("field_0", 7).unwrap();
    data.set_i32("field_1", 8).unwrap();

    let bytes = dynamic_data_to_cdr(&data, &discovered.topic_descriptor).unwrap();
    let back = cdr_to_dynamic_data(
        &bytes,
        &discovered.type_schema,
        &discovered.topic_descriptor,
    )
    .unwrap();

    assert_eq!(back.get_i32("field_0").unwrap(), 7);
    assert_eq!(back.get_i32("field_1").unwrap(), 8);
}

#[test]
fn public_string_cdr_round_trip_preserves_value() {
    // Given: a discovered public type containing one unbounded string.
    let participant = DomainParticipant::new(0).unwrap();
    let discovered = discovered(&participant, |participant, ty| {
        let string_type = DynamicTypeBuilder::unbounded_string8()
            .build(participant)
            .unwrap();
        ty.add_member(DynamicMemberBuilder::new("text", string_type.as_spec()).id(1))
            .unwrap();
    });
    let mut data = DynamicData::new(&discovered.type_schema);
    data.set_string("field_0", "valid").unwrap();

    // When: the public serializer and deserializer perform a complete round-trip.
    let bytes = dynamic_data_to_cdr(&data, &discovered.topic_descriptor).unwrap();
    let decoded = cdr_to_dynamic_data(
        &bytes,
        &discovered.type_schema,
        &discovered.topic_descriptor,
    )
    .unwrap();

    // Then: the exact value survives and the fixture has a length header plus NUL.
    assert_eq!(decoded.get_string("field_0").unwrap(), "valid");
    assert_eq!(u32::from_ne_bytes(bytes[..4].try_into().unwrap()), 6);
    assert_eq!(bytes.last(), Some(&0));
}

fn public_string_cdr_fixture(participant: &DomainParticipant) -> (DiscoveredType, Vec<u8>) {
    let discovered = discovered(participant, |participant, ty| {
        let string_type = DynamicTypeBuilder::unbounded_string8()
            .build(participant)
            .unwrap();
        ty.add_member(DynamicMemberBuilder::new("text", string_type.as_spec()).id(1))
            .unwrap();
    });
    let mut data = DynamicData::new(&discovered.type_schema);
    data.set_string("field_0", "valid").unwrap();
    let bytes = dynamic_data_to_cdr(&data, &discovered.topic_descriptor).unwrap();
    (discovered, bytes)
}

#[test]
fn public_dynamic_cdr_rejects_every_truncation_before_native_decode() {
    // Given: every strict prefix of a valid string sample, including 0..4-byte headers.
    let participant = DomainParticipant::new(0).unwrap();
    let (discovered, bytes) = public_string_cdr_fixture(&participant);

    // When: each truncated sample crosses the public decode boundary.
    let results: Vec<_> = (0..bytes.len())
        .map(|len| {
            cdr_to_dynamic_data(
                &bytes[..len],
                &discovered.type_schema,
                &discovered.topic_descriptor,
            )
        })
        .collect();

    // Then: every boundary returns a typed error instead of reaching unsafe C reads.
    for (len, result) in results.iter().enumerate() {
        assert!(
            matches!(result, Err(DdsError::BadParameter(_))),
            "truncation at byte {len} was accepted as {result:?}"
        );
    }
}

#[test]
fn public_dynamic_cdr_rejects_hostile_string_lengths_and_terminator() {
    // Given: valid storage carrying overflow-sized lengths or no string terminator.
    let participant = DomainParticipant::new(0).unwrap();
    let (discovered, bytes) = public_string_cdr_fixture(&participant);
    let mut oversized = bytes.clone();
    oversized[..4].copy_from_slice(&u32::MAX.to_ne_bytes());
    let mut wrapping = bytes.clone();
    wrapping[..4].copy_from_slice(&(u32::MAX - 3).to_ne_bytes());
    let mut unterminated = bytes;
    *unterminated.last_mut().unwrap() = b'!';

    // When: the malformed corpus crosses the public decode boundary.
    let results: Vec<_> = [oversized, wrapping, unterminated]
        .iter()
        .map(|input| {
            cdr_to_dynamic_data(input, &discovered.type_schema, &discovered.topic_descriptor)
        })
        .collect();

    // Then: all variants fail with the public malformed-input error.
    assert!(results
        .iter()
        .all(|result| matches!(result, Err(DdsError::BadParameter(_)))));
}

#[test]
fn public_dynamic_cdr_rejects_bytes_for_the_wrong_descriptor() {
    // Given: four valid Int32 CDR bytes submitted against a string descriptor.
    let participant = DomainParticipant::new(0).unwrap();
    let (string_discovered, _) = public_string_cdr_fixture(&participant);
    let integer_discovered = discovered(&participant, |_, ty| {
        ty.add_member(DynamicMemberBuilder::primitive("value", DynamicPrimitiveKind::Int32).id(1))
            .unwrap();
    });
    let mut integer = DynamicData::new(&integer_discovered.type_schema);
    integer.set_i32("field_0", 7).unwrap();
    let bytes = dynamic_data_to_cdr(&integer, &integer_discovered.topic_descriptor).unwrap();

    // When: the public decoder validates the bytes with the string descriptor.
    let result = cdr_to_dynamic_data(
        &bytes,
        &string_discovered.type_schema,
        &string_discovered.topic_descriptor,
    );

    // Then: the descriptor mismatch is rejected as malformed CDR.
    assert!(matches!(result, Err(DdsError::BadParameter(_))));
}

#[test]
fn public_dynamic_cdr_cleans_native_members_when_schema_validation_fails() {
    // Given: valid string CDR and a deliberately incompatible Int32 schema.
    let participant = DomainParticipant::new(0).unwrap();
    let (string_discovered, bytes) = public_string_cdr_fixture(&participant);
    let integer_discovered = discovered(&participant, |_, ty| {
        ty.add_member(DynamicMemberBuilder::primitive("value", DynamicPrimitiveKind::Int32).id(1))
            .unwrap();
    });

    // When: post-read DynamicData validation fails repeatedly after C allocated strings.
    let results: Vec<_> = (0..64)
        .map(|_| {
            cdr_to_dynamic_data(
                &bytes,
                &integer_discovered.type_schema,
                &string_discovered.topic_descriptor,
            )
        })
        .collect();

    // Then: every attempt returns an error; ASan verifies each native member is freed once.
    assert!(results.iter().all(Result::is_err));
}

/// A key name with an interior NUL must be reported, not leak the native buffer
/// it was allocated alongside. (The allocation now happens after this fallible
/// step; there is no `Drop` on a raw pointer to clean it up otherwise.)
#[test]
fn serialisation_of_a_plain_type_does_not_leak_on_success() {
    let participant = DomainParticipant::new(0).unwrap();
    let discovered = discovered(&participant, |_, ty| {
        ty.add_member(DynamicMemberBuilder::primitive("v", DynamicPrimitiveKind::Int32).id(1))
            .unwrap();
    });

    // Repeat enough that a per-call leak of the native sample buffer shows up
    // under ASan as a growing set of unfreed allocations.
    for i in 0..64 {
        let mut data = DynamicData::new(&discovered.type_schema);
        data.set_i32("field_0", i).unwrap();
        let bytes = dynamic_data_to_cdr(&data, &discovered.topic_descriptor).unwrap();
        let back = cdr_to_dynamic_data(
            &bytes,
            &discovered.type_schema,
            &discovered.topic_descriptor,
        )
        .unwrap();
        assert_eq!(back.get_i32("field_0").unwrap(), i);
    }
}
