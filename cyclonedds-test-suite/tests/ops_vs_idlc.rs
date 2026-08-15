//! Differential coverage for the ops arrays the derive generates.
//!
//! The reference is the C `idlc` from Eclipse Cyclone DDS 11.0.1, run over
//! `tests/idl/ops_reference.idl`; `scripts/regen-ops-fixtures.sh` rebuilds it and
//! prints the arrays. The expectations below are transcribed from that output
//! with `offsetof`/`sizeof` expressed against `<T as DdsType>::Native`, so they
//! stay valid on any target.
//!
//! This exists because reading the emitter was already shown to be insufficient:
//! the width-table defect (`a2bfb2c`) survived several readings, and every defect
//! recorded below was invisible to inspection until idlc was put next to it.
//!
//! Two documented differences from idlc, neither a defect:
//!
//! * idlc appends a `KOF` chain to `m_ops` for keys; this crate builds the same
//!   chain in `Topic::new` from `DdsType::keys()`, so `ops()` itself stops at the
//!   terminating `RTS`. The `ops_path` values are compared against idlc's `KOF`
//!   operands instead.
//! * idlc emits one shared sub-ops block when two members have the same element
//!   type; the derive emits one block per member. Both are valid — the jump
//!   offsets are per-instruction — so `TwoSeqs` is checked structurally rather
//!   than word-for-word.

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, DdsTypeDerive)]
struct P {
    x: i32,
    y: i32,
}

const ADR_I32: u32 = OP_ADR | TYPE_4BY | OP_FLAG_SGN;
const ADR_KEY_I32: u32 = OP_ADR | OP_FLAG_KEY | TYPE_4BY | OP_FLAG_SGN;

/// `P::ops()` — the block every case below embeds.
fn p_ops() -> Vec<u32> {
    type PN = <P as DdsType>::Native;
    vec![
        ADR_I32,
        std::mem::offset_of!(PN, x) as u32,
        ADR_I32,
        std::mem::offset_of!(PN, y) as u32,
        OP_RTS,
    ]
}

// ── a sequence of composites, followed by another member ────────────────────

/// ```idl
/// struct SeqMid { long h; sequence<P> items; long tail; };
/// ```
/// idlc:
/// ```text
///   ADR|4BY|SGN, offsetof(h)
///   ADR|SEQ|STU, offsetof(items), sizeof(P), (4u<<16u) + 7u
///   ADR|4BY|SGN, offsetof(tail)
///   RTS
///   <P ops>
/// ```
/// Both halves of the jump word are computed: the high half is the width of the
/// SEQ instruction (4), the low half the distance from it to the element's
/// sub-ops — 7 here, because `tail` and the `RTS` sit in between. Emitting a
/// constant `+5` and placing the sub-ops inline is correct only when the
/// sequence is the last member.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct SeqMid {
    h: i32,
    items: Vec<P>,
    tail: i32,
}

#[test]
fn ops_seq_then_member_match_idlc() {
    type N = <SeqMid as DdsType>::Native;
    let mut expected = vec![
        ADR_I32,
        std::mem::offset_of!(N, h) as u32,
        OP_ADR | TYPE_SEQ | SUBTYPE_STU,
        std::mem::offset_of!(N, items) as u32,
        std::mem::size_of::<<P as DdsType>::Native>() as u32,
        (4u32 << 16) + 7u32,
        ADR_I32,
        std::mem::offset_of!(N, tail) as u32,
        OP_RTS,
    ];
    expected.extend(p_ops());

    assert_eq!(SeqMid::ops(), expected);
}

#[test]
fn member_after_composite_sequence_round_trips() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<SeqMid>(&unique_topic("ops_seq_mid"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    let sent = SeqMid {
        h: 1,
        items: vec![P { x: 1, y: 2 }, P { x: 3, y: 4 }],
        tail: 0x5EED,
    };
    writer.write(&sent).unwrap();

    assert!(wait_for(Duration::from_secs(3), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));

    let samples = reader.take().unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(
        samples[0].tail, sent.tail,
        "the member following a Vec<Composite> was not serialized"
    );
    assert_eq!(samples[0].items, sent.items);
}

// ── bounded sequence and array of composites, followed by another member ────

/// ```idl
/// struct BSeqMid { long h; sequence<P, 4> items; long tail; };
/// ```
/// idlc: `ADR|BSQ|STU, offsetof, 4u, sizeof(P), (5u<<16u) + 8u`
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct BSeqMid {
    h: i32,
    items: DdsBoundedSequence<P, 4>,
    tail: i32,
}

#[test]
fn ops_bounded_seq_then_member_match_idlc() {
    type N = <BSeqMid as DdsType>::Native;
    let mut expected = vec![
        ADR_I32,
        std::mem::offset_of!(N, h) as u32,
        OP_ADR | TYPE_BSQ | SUBTYPE_STU,
        std::mem::offset_of!(N, items) as u32,
        4u32,
        std::mem::size_of::<<P as DdsType>::Native>() as u32,
        (5u32 << 16) + 8u32,
        ADR_I32,
        std::mem::offset_of!(N, tail) as u32,
        OP_RTS,
    ];
    expected.extend(p_ops());

    assert_eq!(BSeqMid::ops(), expected);
}

/// ```idl
/// struct ArrMid { long h; P items[3]; long tail; };
/// ```
/// idlc: `ADR|ARR|STU, offsetof, 3u, (5u<<16u) + 8u, sizeof(P)` — note the jump
/// word comes *before* the element size for arrays, unlike sequences.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct ArrMid {
    h: i32,
    items: [P; 3],
    tail: i32,
}

#[test]
fn ops_composite_array_then_member_match_idlc() {
    type N = <ArrMid as DdsType>::Native;
    let mut expected = vec![
        ADR_I32,
        std::mem::offset_of!(N, h) as u32,
        OP_ADR | TYPE_ARR | SUBTYPE_STU,
        std::mem::offset_of!(N, items) as u32,
        3u32,
        (5u32 << 16) + 8u32,
        std::mem::size_of::<<P as DdsType>::Native>() as u32,
        ADR_I32,
        std::mem::offset_of!(N, tail) as u32,
        OP_RTS,
    ];
    expected.extend(p_ops());

    assert_eq!(ArrMid::ops(), expected);
}

// ── nested struct followed by another member ────────────────────────────────

/// ```idl
/// struct NestMid { long h; P inner; long tail; };
/// ```
/// idlc: `ADR|EXT, offsetof(inner), (3u<<16u) + 6u`
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct NestMid {
    h: i32,
    inner: P,
    tail: i32,
}

#[test]
fn ops_nested_struct_then_member_match_idlc() {
    type N = <NestMid as DdsType>::Native;
    let mut expected = vec![
        ADR_I32,
        std::mem::offset_of!(N, h) as u32,
        OP_ADR | TYPE_EXT,
        std::mem::offset_of!(N, inner) as u32,
        (3u32 << 16) + 6u32,
        ADR_I32,
        std::mem::offset_of!(N, tail) as u32,
        OP_RTS,
    ];
    expected.extend(p_ops());

    assert_eq!(NestMid::ops(), expected);
}

// ── key indices after a composite member ────────────────────────────────────

/// ```idl
/// struct KeyAfterNested { P inner; @key long id; };
/// ```
/// idlc closes with `DDS_OP_KOF | 1, 3u` — the key's ADR sits at word 3, right
/// after the three words of the `ADR|EXT` instruction. The sub-ops block does
/// not occupy indices in the main instruction stream.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct KeyAfterNested {
    inner: P,
    #[key]
    id: i32,
}

#[test]
fn key_index_after_nested_struct_matches_idlc() {
    let keys = KeyAfterNested::keys();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].name, "id");
    assert_eq!(keys[0].ops_path, vec![3u32]);

    // and that index really does point at the key's ADR instruction
    let ops = KeyAfterNested::ops();
    assert_eq!(ops[keys[0].ops_path[0] as usize], ADR_KEY_I32);
}

/// ```idl
/// struct KeyAfterSeq { sequence<P> items; @key long id; };
/// ```
/// idlc closes with `DDS_OP_KOF | 1, 4u`.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct KeyAfterSeq {
    items: Vec<P>,
    #[key]
    id: i32,
}

#[test]
fn key_index_after_composite_sequence_matches_idlc() {
    let keys = KeyAfterSeq::keys();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].name, "id");
    assert_eq!(keys[0].ops_path, vec![4u32]);

    let ops = KeyAfterSeq::ops();
    assert_eq!(ops[keys[0].ops_path[0] as usize], ADR_KEY_I32);
}

// ── two sequences of the same element type ──────────────────────────────────

/// ```idl
/// struct TwoSeqs { sequence<P> a1; sequence<P> a2; long tail; };
/// ```
/// idlc shares one `P` block: `(4u<<16u) + 11u` then `(4u<<16u) + 7u`, both
/// landing on the same words. The derive emits a block per member, so only the
/// structure is asserted: each jump must land on a well-formed `P` block, and
/// `tail` must be reachable at +4 from the second sequence.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct TwoSeqs {
    a1: Vec<P>,
    a2: Vec<P>,
    tail: i32,
}

#[test]
fn two_composite_sequences_each_jump_to_a_valid_block() {
    let ops = TwoSeqs::ops();
    let seq_head = OP_ADR | TYPE_SEQ | SUBTYPE_STU;

    let starts: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, &w)| w == seq_head)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(starts.len(), 2, "expected two sequence instructions");

    for start in starts {
        let jump = ops[start + 3];
        assert_eq!(jump >> 16, 4, "sequence instruction is 4 words wide");
        let child = start + (jump & 0xFFFF) as usize;
        assert_eq!(
            &ops[child..child + p_ops().len()],
            p_ops().as_slice(),
            "jump from word {start} does not land on a P block"
        );
    }

    // `tail` is the member after the second sequence: +4 from its instruction.
    let second = ops
        .iter()
        .enumerate()
        .filter(|(_, &w)| w == seq_head)
        .map(|(i, _)| i)
        .nth(1)
        .unwrap();
    assert_eq!(ops[second + 4], ADR_I32, "tail is not reachable from a2");
}

#[test]
fn two_composite_sequences_round_trip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<TwoSeqs>(&unique_topic("ops_two_seqs"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    let sent = TwoSeqs {
        a1: vec![P { x: 1, y: 2 }],
        a2: vec![P { x: 3, y: 4 }, P { x: 5, y: 6 }],
        tail: 99,
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
