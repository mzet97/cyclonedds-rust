//! Regression coverage for `Loan<T>` over types with heap-allocated fields.
//!
//! Every pre-existing loan test used POD types only (`struct { id: i32 }`),
//! where `T::Native` happens to be layout-identical to `T`. That is why
//! `Loan::iter()` shipped materializing `&*(ptr as *const T)` over memory whose
//! real layout is `T::Native`: for a type with `String`/`Vec` fields the DDS
//! buffer holds `DdsString` (8 bytes) / `DdsSequence` where `T` expects
//! `String` (24 bytes) / `Vec` — an out-of-bounds read, and heap corruption on
//! `to_vec()` because `String::clone` then allocates from a garbage capacity
//! and frees a pointer Rust never allocated.
//!
//! These are the cases that must be exercised under AddressSanitizer:
//! ```bash
//! RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p cyclonedds-test-suite \
//!     --target x86_64-unknown-linux-gnu --test loan_heap_fields
//! ```

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct HeapMessage {
    #[key]
    id: i32,
    text: String,
    values: Vec<i32>,
}

fn sample(id: i32) -> HeapMessage {
    HeapMessage {
        id,
        text: format!("loan-heap-{id}"),
        values: vec![id, id * 2, id * 3],
    }
}

#[test]
fn read_loan_iter_yields_intact_heap_fields() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<HeapMessage>(&unique_topic("loan_heap_read"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer.write(&sample(1)).unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));

    let loan = reader.read_loan().unwrap();
    assert_eq!(loan.len(), 1);

    let observed: Vec<(i32, String, Vec<i32>)> = loan
        .iter()
        .map(|s| s.expect("sample failed to decode"))
        .map(|s| (s.data.id, s.data.text.clone(), s.data.values.clone()))
        .collect();

    assert_eq!(observed[0].0, 1);
    assert_eq!(observed[0].1, "loan-heap-1");
    assert_eq!(observed[0].2, vec![1, 2, 3]);
}

#[test]
fn take_loan_to_vec_does_not_corrupt_heap() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<HeapMessage>(&unique_topic("loan_heap_take"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    for i in 1..=4 {
        writer.write(&sample(i)).unwrap();
    }

    assert!(wait_for(Duration::from_secs(3), || reader
        .read()
        .unwrap_or_default()
        .len()
        >= 4));

    // `to_vec` clones every sample out of the loan. With the layout bug this is
    // where `String::clone` reads a garbage (ptr, cap, len) triple.
    let owned = {
        let loan = reader.take_loan().unwrap();
        assert_eq!(loan.len(), 4);
        loan.to_vec().expect("to_vec failed")
    };

    let mut ids: Vec<i32> = owned.iter().map(|s| s.data.id).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec![1, 2, 3, 4]);

    for s in &owned {
        assert_eq!(s.data.text, format!("loan-heap-{}", s.data.id));
        assert_eq!(s.data.values, vec![s.data.id, s.data.id * 2, s.data.id * 3]);
    }

    // Dropping `owned` here must not double-free: with the bug the cloned
    // Strings hold pointers into CycloneDDS-owned memory.
    drop(owned);
}

#[test]
fn peek_with_heap_fields_is_sound() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<HeapMessage>(&unique_topic("loan_heap_peek"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer.write(&sample(9)).unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .peek()
        .unwrap()
        .is_empty()));

    let peeked = reader.peek().unwrap();
    let first = peeked
        .iter()
        .next()
        .unwrap()
        .expect("sample failed to decode");
    assert_eq!(first.data.id, 9);
    assert_eq!(first.data.text, "loan-heap-9");
}
