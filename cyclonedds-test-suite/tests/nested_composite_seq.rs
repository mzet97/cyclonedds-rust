//! Regression coverage for `Vec<Composite>` where the composite has heap fields.
//!
//! The gap `writer.rs` admitted in prose: the derive used the inner *Rust* type
//! as the `DdsSequence` element type and as the sequence's element stride,
//! instead of `<Inner as DdsType>::Native`. Correct only while `Inner` is POD.
//!
//! Once `Inner` owns a `String`, the two disagree: `size_of::<Inner>()` counts a
//! 24-byte `String`, `size_of::<Inner::Native>()` counts an 8-byte `DdsString`.
//! The ops array then tells CycloneDDS to walk the sequence buffer with the
//! wrong stride, and `from_slice` fills that buffer with Rust `String` triples
//! that the C side dereferences as `char *`.
//!
//! Ground truth for `expected_ops` is the C `idlc` (11.0.1) output for
//! `sequence<Inner> items;` — see `tests/idl/` and `scripts/regen-ops-fixtures.sh`.
//!
//! Run under AddressSanitizer to catch the read side:
//! ```bash
//! RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p cyclonedds-test-suite \
//!     --target x86_64-unknown-linux-gnu --test nested_composite_seq
//! ```

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct Inner {
    name: String,
    v: i32,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct Outer {
    #[key]
    id: i32,
    items: Vec<Inner>,
}

/// A POD inner type, to prove the fix does not disturb the case that already
/// worked — for `PodInner`, `Native` *is* `Self` and the stride is unchanged.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct PodInner {
    a: i32,
    b: i32,
}

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct PodOuter {
    #[key]
    id: i32,
    items: Vec<PodInner>,
}

/// Locate the `OP_ADR | TYPE_SEQ | SUBTYPE_STU` instruction and return the
/// element-size word that follows the offset word.
fn sequence_element_stride(ops: &[u32]) -> u32 {
    let head = OP_ADR | TYPE_SEQ | SUBTYPE_STU;
    let pos = ops
        .iter()
        .position(|&w| w == head)
        .expect("no ADR|SEQ|STU instruction in ops()");
    // layout: [head, offset, element_size, (4<<16)+5, RTS, ...inner ops..., RTS]
    ops[pos + 2]
}

#[test]
fn sequence_of_heap_composite_uses_native_stride() {
    let ops = Outer::ops();
    let stride = sequence_element_stride(&ops);

    assert_eq!(
        stride as usize,
        std::mem::size_of::<<Inner as DdsType>::Native>(),
        "the sequence stride must be the size of the wire layout \
         (Inner::Native), not of the Rust type; ops = {ops:?}"
    );

    // And the two really are different for this type — otherwise the assertion
    // above would pass for the wrong reason.
    assert_ne!(
        std::mem::size_of::<Inner>(),
        std::mem::size_of::<<Inner as DdsType>::Native>(),
        "Inner must have a distinct native layout for this test to mean anything"
    );
}

#[test]
fn sequence_of_pod_composite_stride_is_unchanged() {
    let ops = PodOuter::ops();
    let stride = sequence_element_stride(&ops);

    assert_eq!(stride as usize, std::mem::size_of::<PodInner>());
    assert_eq!(
        std::mem::size_of::<PodInner>(),
        std::mem::size_of::<<PodInner as DdsType>::Native>(),
    );
}

/// Word-by-word against the C `idlc` output for the equivalent IDL.
///
/// ```idl
/// struct Inner { string name; long v; };
/// struct Outer { @key long id; sequence<Inner> items; };
/// ```
#[test]
fn ops_match_idlc_for_sequence_of_heap_composite() {
    let native_stride = std::mem::size_of::<<Inner as DdsType>::Native>() as u32;
    let inner_native_name_off =
        std::mem::offset_of!(<Inner as DdsType>::Native, name) as u32;
    let inner_native_v_off = std::mem::offset_of!(<Inner as DdsType>::Native, v) as u32;
    let outer_native_id_off = std::mem::offset_of!(<Outer as DdsType>::Native, id) as u32;
    let outer_native_items_off =
        std::mem::offset_of!(<Outer as DdsType>::Native, items) as u32;

    // idlc emits, for the sequence member:
    //   ADR|SEQ|STU, offsetof(Outer, items), sizeof(Inner), (4<<16)+5
    //   RTS
    //   ADR|STR, offsetof(Inner, name)
    //   ADR|4BY|SGN, offsetof(Inner, v)
    //   RTS
    let expected = vec![
        OP_ADR | OP_FLAG_KEY | TYPE_4BY | OP_FLAG_SGN,
        outer_native_id_off,
        OP_ADR | TYPE_SEQ | SUBTYPE_STU,
        outer_native_items_off,
        native_stride,
        (4u32 << 16) + 5u32,
        OP_RTS,
        OP_ADR | TYPE_STR,
        inner_native_name_off,
        OP_ADR | TYPE_4BY | OP_FLAG_SGN,
        inner_native_v_off,
        OP_RTS,
    ];

    assert_eq!(Outer::ops(), expected);
}

#[test]
fn sequence_of_heap_composite_round_trips() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<Outer>(&unique_topic("nested_composite_seq"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let sent = Outer {
        id: 1,
        items: vec![
            Inner {
                name: "first".into(),
                v: 10,
            },
            Inner {
                name: "second-and-a-longer-one".into(),
                v: 20,
            },
        ],
    };
    writer.write(&sent).unwrap();

    assert!(wait_for(Duration::from_secs(3), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));

    let samples = reader.take().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0], sent);
}
