//! The `ops()` scanner in `cyclonedds-derive` must agree with CycloneDDS's
//! instruction encoding.
//!
//! The generated `ops()` walks the instruction stream it just built to collect
//! the positions of `TYPE_EXT` (nested composite) instructions, then patches
//! each one with the jump offset to its child block. Walking requires knowing
//! how many words every instruction occupies. Several of those word counts
//! disagree with `dds_opcodes.h`, e.g.
//!
//! | instruction     | dds_opcodes.h | scanner |
//! |-----------------|---------------|---------|
//! | `ADR SEQ ENU`   | 3             | 2       |
//! | `ADR SEQ BST`   | 3             | 4       |
//! | `ADR ARR ENU`   | 4             | 3       |
//! | `ADR BSQ ENU`   | 4             | 3       |
//!
//! A miscount only matters when a mis-sized instruction is followed by a nested
//! composite: the scan lands mid-instruction and either misses the real
//! `TYPE_EXT` (leaving its jump word unpatched) or mistakes a data word for one
//! and patches that instead. Every pre-existing test either used a mis-sized
//! field *or* a nested struct, never both — which is why this went unnoticed.
//!
//! Each type below pairs one suspect field with a trailing nested struct.

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::time::Duration;

#[repr(i32)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, DdsEnumDerive)]
enum Level {
    Low = 0,
    Mid = 1,
    High = 2,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct Inner {
    a: i32,
    b: f64,
}

/// `Vec<enum>` (ADR SEQ ENU, 3 words) followed by a nested struct.
#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct SeqEnumThenNested {
    #[key]
    id: i32,
    #[dds_enum]
    levels: Vec<Level>,
    inner: Inner,
}

/// `[enum; N]` (ADR ARR ENU, 4 words) followed by a nested struct.
#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct ArrEnumThenNested {
    #[key]
    id: i32,
    #[dds_enum]
    levels: [Level; 3],
    inner: Inner,
}

/// Bounded sequence of enum (ADR BSQ ENU, 4 words) followed by a nested struct.
#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct BsqEnumThenNested {
    #[key]
    id: i32,
    #[dds_enum]
    levels: DdsBoundedSequence<Level, 4>,
    inner: Inner,
}

/// Bounded string inside a sequence (ADR SEQ BST, 3 words) then a nested struct.
#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct SeqStringThenNested {
    #[key]
    id: i32,
    names: Vec<String>,
    inner: Inner,
}

fn roundtrip<T>(topic_hint: &str, sample: T, check: impl Fn(&T))
where
    T: DdsType + Clone + std::fmt::Debug,
{
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<T>(&unique_topic(topic_hint))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer.write(&sample).unwrap();

    assert!(
        wait_for(Duration::from_secs(2), || !reader
            .read()
            .unwrap_or_default()
            .is_empty()),
        "sample never arrived for {topic_hint}"
    );

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    check(&taken[0]);
}

#[test]
fn seq_enum_followed_by_nested_struct() {
    roundtrip(
        "ops_seq_enum_nested",
        SeqEnumThenNested {
            id: 1,
            levels: vec![Level::Low, Level::High],
            inner: Inner { a: 42, b: 2.5 },
        },
        |got| {
            assert_eq!(got.id, 1);
            assert_eq!(got.levels, vec![Level::Low, Level::High]);
            assert_eq!(got.inner.a, 42, "nested struct corrupted");
            assert!((got.inner.b - 2.5).abs() < f64::EPSILON, "nested struct corrupted");
        },
    );
}

#[test]
fn arr_enum_followed_by_nested_struct() {
    roundtrip(
        "ops_arr_enum_nested",
        ArrEnumThenNested {
            id: 2,
            levels: [Level::Low, Level::Mid, Level::High],
            inner: Inner { a: 7, b: -1.25 },
        },
        |got| {
            assert_eq!(got.id, 2);
            assert_eq!(got.levels, [Level::Low, Level::Mid, Level::High]);
            assert_eq!(got.inner.a, 7, "nested struct corrupted");
            assert!((got.inner.b + 1.25).abs() < f64::EPSILON, "nested struct corrupted");
        },
    );
}

#[test]
fn bsq_enum_followed_by_nested_struct() {
    roundtrip(
        "ops_bsq_enum_nested",
        BsqEnumThenNested {
            id: 3,
            levels: DdsBoundedSequence::from_slice(&[Level::Mid, Level::High]).unwrap(),
            inner: Inner { a: 99, b: 0.5 },
        },
        |got| {
            assert_eq!(got.id, 3);
            assert_eq!(got.levels.to_vec(), vec![Level::Mid, Level::High]);
            assert_eq!(got.inner.a, 99, "nested struct corrupted");
            assert!((got.inner.b - 0.5).abs() < f64::EPSILON, "nested struct corrupted");
        },
    );
}

#[test]
fn seq_string_followed_by_nested_struct() {
    roundtrip(
        "ops_seq_string_nested",
        SeqStringThenNested {
            id: 4,
            names: vec!["alpha".to_string(), "beta".to_string()],
            inner: Inner { a: -5, b: 3.75 },
        },
        |got| {
            assert_eq!(got.id, 4);
            assert_eq!(got.names, vec!["alpha".to_string(), "beta".to_string()]);
            assert_eq!(got.inner.a, -5, "nested struct corrupted");
            assert!((got.inner.b - 3.75).abs() < f64::EPSILON, "nested struct corrupted");
        },
    );
}
