//! A `#[derive(DdsUnionDerive)]` type over an actual DDS round-trip.
//!
//! Every previous union test drove `clone_out` against a buffer built in the
//! test — `union_unknown_discriminator.rs` fills a `Vec<u8>` by hand. None of
//! them ever handed a union to CycloneDDS, so `DdsUnionType::ops()` had no
//! coverage at all: the same shape as the union derive defect fixed in
//! `62b1afd`, which had never compiled for a non-String case and nobody noticed.
//!
//! The reference is what the C `idlc` emits for the equivalent IDL:
//!
//! ```idl
//! union MyUnion switch (long) {
//!   case 1: long a;
//!   case 2: double b;
//!   default: long c;
//! };
//! ```
//! ```text
//! ADR|FLAG_MU|TYPE_UNI|SUBTYPE_4BY|FLAG_DEF|FLAG_SGN, offsetof(_d), 3u, (16u<<16u)+4u
//! JEQ4|TYPE_4BY, 1, offsetof(_u.a), 0u
//! JEQ4|TYPE_8BY, 2, offsetof(_u.b), 0u
//! JEQ4|TYPE_4BY, 0, offsetof(_u.c), 0u
//! RTS
//! ```
//!
//! A `JEQ4` entry is four words — opcode carrying the member's *type*, the
//! discriminator value, the member's offset, then a trailing word (0 for
//! primitives, the max literal for enums). `dds_opcodes.h` spells both forms
//! out:
//!
//! ```text
//! [JEQ,  nBY, 0] [disc] [offset]
//! [JEQ4, e | nBY, 0] [disc] [offset] 0
//! ```

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsUnionDerive)]
#[dds_discriminant(u32)]
enum Choice {
    #[dds_case(1)]
    AsI32(i32),
    #[dds_case(2)]
    AsF64(f64),
}

#[test]
fn union_ops_carry_the_member_type_and_offset() {
    let ops = Choice::ops();

    // The header is four words: ADR|UNI|<disc subtype>, offset, case count, jump.
    assert!(ops.len() >= 4, "union ops too short: {ops:?}");
    assert_eq!(
        ops[0] & DDS_OP_MASK_CONST,
        OP_ADR,
        "first word is not an ADR: {ops:?}"
    );
    assert_eq!(
        ops[0] & DDS_OP_TYPE_MASK_CONST,
        TYPE_UNI,
        "first word is not a union: {ops:?}"
    );
    assert_eq!(ops[2], 2, "case count should be 2: {ops:?}");

    // Each JEQ4 is four words and the opcode must carry the member's type:
    // without it the interpreter cannot know how to read the case.
    let first_case = (ops[3] & 0xFFFF) as usize;
    assert_eq!(first_case, 4, "first case should start at word 4: {ops:?}");

    let jeq4_i32 = ops[first_case];
    assert_eq!(
        jeq4_i32 & DDS_OP_MASK_CONST,
        OP_JEQ4,
        "expected a JEQ4 at word {first_case}: {ops:?}"
    );
    assert_ne!(
        jeq4_i32 & DDS_OP_TYPE_MASK_CONST,
        0,
        "the JEQ4 opcode carries no member type, so the case cannot be read: {ops:?}"
    );
    assert_eq!(ops[first_case + 1], 1, "case value: {ops:?}");

    let jeq4_f64 = ops[first_case + 4];
    assert_eq!(
        jeq4_f64 & DDS_OP_MASK_CONST,
        OP_JEQ4,
        "the second JEQ4 is not four words after the first: {ops:?}"
    );
    assert_eq!(ops[first_case + 5], 2, "case value: {ops:?}");

    // Header jump: high half is the distance to the next member instruction,
    // which for a two-case union is 4 + 2*4 = 12.
    assert_eq!(ops[3] >> 16, 12, "header jump width: {ops:?}");
}

#[test]
fn union_round_trips_over_dds() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<Choice>(&unique_topic("union_wire"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer.write(&Choice::AsI32(0x5EED)).unwrap();

    assert!(wait_for(Duration::from_secs(3), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));

    let samples = reader.take().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0], Choice::AsI32(0x5EED));
}

#[test]
fn union_round_trips_the_other_arm() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<Choice>(&unique_topic("union_wire_f64"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer.write(&Choice::AsF64(2.5)).unwrap();

    assert!(wait_for(Duration::from_secs(3), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));

    let samples = reader.take().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0], Choice::AsF64(2.5));
}
