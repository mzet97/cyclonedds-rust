//! The `Native` translation has to be applied recursively, in every shape a
//! composite member can take.
//!
//! `Vec<Inner>` is the case `writer.rs` called out and `nested_composite_seq.rs`
//! covers. The same reasoning applies to the other four, which had the same
//! defect for the same reason: the generated native struct kept the *Rust* type
//! of the member, while the sub-ops emitted for it address the member through
//! `Inner::Native`. As long as `Inner` was POD the two coincided and nothing
//! showed.
//!
//! Each type here has an `Inner` with a `String`, so `size_of::<Inner>()` (32 on
//! 64-bit: `String` is 24) and `size_of::<Inner::Native>()` (16: `DdsString` is
//! 8) disagree and the layouts cannot silently match.
//!
//! Worth running under AddressSanitizer:
//! ```bash
//! RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p cyclonedds-test-suite \
//!     --target x86_64-unknown-linux-gnu --test native_layout_recursive
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

fn inner(name: &str, v: i32) -> Inner {
    Inner {
        name: name.to_string(),
        v,
    }
}

/// Sanity: the whole point of these tests is that the two layouts differ.
#[test]
fn inner_layouts_differ() {
    assert_ne!(
        std::mem::size_of::<Inner>(),
        std::mem::size_of::<<Inner as DdsType>::Native>(),
    );
}

// ── nested struct ───────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct WithNested {
    #[key]
    id: i32,
    inner: Inner,
    tail: i32,
}

#[test]
fn nested_heap_composite_native_layout() {
    type N = <WithNested as DdsType>::Native;

    // The member is held as `Inner::Native`, so the enclosing native struct is
    // smaller than the Rust one by exactly the difference between the two inner
    // layouts. Keeping `Inner` here is what made `descriptor_size()` describe a
    // struct the sub-ops did not address.
    assert_eq!(
        std::mem::size_of::<WithNested>() - std::mem::size_of::<N>(),
        std::mem::size_of::<Inner>() - std::mem::size_of::<<Inner as DdsType>::Native>(),
    );
    assert_eq!(
        WithNested::descriptor_size() as usize,
        std::mem::size_of::<N>()
    );
}

#[test]
fn nested_heap_composite_round_trips() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<WithNested>(&unique_topic("native_nested"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    let sent = WithNested {
        id: 1,
        inner: inner("nested-heap-field", 42),
        tail: 0x1234,
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

// ── DdsSequence<Inner> declared directly ────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct WithDdsSequence {
    #[key]
    id: i32,
    items: DdsSequence<Inner>,
    tail: i32,
}

#[test]
fn dds_sequence_of_heap_composite_round_trips() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<WithDdsSequence>(&unique_topic("native_ddsseq"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    let sent = WithDdsSequence {
        id: 2,
        items: DdsSequence::from_slice(&[inner("one", 1), inner("two-longer", 2)]).unwrap(),
        tail: 0x2345,
    };
    writer.write(&sent).unwrap();

    assert!(wait_for(Duration::from_secs(3), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));

    let samples = reader.take().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].id, sent.id);
    assert_eq!(samples[0].tail, sent.tail);
    assert_eq!(samples[0].items.as_slice(), sent.items.as_slice());
}

// ── DdsBoundedSequence<Inner, N> ────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct WithBoundedSequence {
    #[key]
    id: i32,
    items: DdsBoundedSequence<Inner, 4>,
    tail: i32,
}

#[test]
fn bounded_sequence_of_heap_composite_round_trips() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<WithBoundedSequence>(&unique_topic("native_bseq"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    let sent = WithBoundedSequence {
        id: 3,
        items: DdsBoundedSequence::from_slice(&[inner("a", 1), inner("bb", 2), inner("ccc", 3)])
            .unwrap(),
        tail: 0x3456,
    };
    writer.write(&sent).unwrap();

    assert!(wait_for(Duration::from_secs(3), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));

    let samples = reader.take().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].id, sent.id);
    assert_eq!(samples[0].tail, sent.tail);
    assert_eq!(samples[0].items.as_slice(), sent.items.as_slice());
}

// ── [Inner; N] ──────────────────────────────────────────────────────────────

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct WithArray {
    #[key]
    id: i32,
    items: [Inner; 3],
    tail: i32,
}

#[test]
fn array_of_heap_composite_round_trips() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<WithArray>(&unique_topic("native_arr"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    let sent = WithArray {
        id: 4,
        items: [inner("first", 1), inner("second", 2), inner("third", 3)],
        tail: 0x4567,
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
