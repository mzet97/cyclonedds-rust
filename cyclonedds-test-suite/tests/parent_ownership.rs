//! A child entity must keep its parent alive (backlog A1/A2/A7).
//!
//! Every entity in this crate stores a bare `dds_entity_t` and retains nothing
//! of the entity it was created from. Two consequences, both reachable from
//! ordinary safe code:
//!
//! 1. **Use after the parent is gone.** `dds_delete` on a participant deletes
//!    its whole subtree, so a `DataReader` that outlives its `DomainParticipant`
//!    is operating on a handle CycloneDDS has already reclaimed. Every call on
//!    it fails.
//!
//! 2. **Drop order.** Struct fields drop in declaration order, so
//!    `struct App { participant, subscriber, reader }` destroys the parent
//!    first and the children's `Drop`s then call `dds_delete` on dead handles.
//!
//! What this is *not*: routinely memory-unsafe. `dds_handle_create` in
//! `dds_handles.c:116` draws each handle uniformly at random from
//! `[1, DDS_MIN_PSEUDO_HANDLE)` — about 2.1e9 values — and every C entry point
//! pins the handle through a hash table, so a stale handle is overwhelmingly
//! likely to be *absent* and return an error rather than resolve to somebody
//! else's entity. Hitting a live entity requires re-drawing that exact value,
//! ~1 in 2.1e9 per entity created. Real, but rare; the everyday defect is the
//! silent error returns these tests measure.

use cyclonedds::*;
use cyclonedds_test_suite::{unique_topic, TestMessage};

/// A reader built inside a scope, escaping it while its participant does not.
///
/// The participant is dropped at the end of the block. If the reader does not
/// own it, CycloneDDS has already deleted the reader by the time we read from
/// it.
#[test]
fn reader_outliving_its_participant_still_works() {
    let topic_name = unique_topic("a1_reader_outlives_dp");

    let reader = {
        let participant = DomainParticipant::new(0).unwrap();
        let topic = Topic::<TestMessage>::new(&participant, &topic_name).unwrap();
        let subscriber = Subscriber::new(&participant).unwrap();
        DataReader::new(&subscriber, &topic).unwrap()
        // participant, subscriber and topic all drop here.
    };

    // Nothing has been written, so an empty take is the correct answer. An
    // error means the entity itself is gone.
    let taken = reader.take();
    assert!(
        taken.is_ok(),
        "reader was deleted with its participant: {:?}",
        taken.err()
    );
}

/// The same for a writer, through a `Publisher`.
#[test]
fn writer_outliving_its_participant_still_works() {
    let topic_name = unique_topic("a1_writer_outlives_dp");

    let writer = {
        let participant = DomainParticipant::new(0).unwrap();
        let topic = Topic::<TestMessage>::new(&participant, &topic_name).unwrap();
        let publisher = Publisher::new(&participant).unwrap();
        DataWriter::new(&publisher, &topic).unwrap()
    };

    let written = writer.write(&TestMessage::new(1, 42, "a1"));
    assert!(
        written.is_ok(),
        "writer was deleted with its participant: {:?}",
        written.err()
    );
}

/// A `Topic` alone must also hold its participant up.
#[test]
fn topic_outliving_its_participant_still_works() {
    let topic_name = unique_topic("a1_topic_outlives_dp");

    let topic = {
        let participant = DomainParticipant::new(0).unwrap();
        Topic::<TestMessage>::new(&participant, &topic_name).unwrap()
    };

    let name = topic.get_name();
    assert!(
        name.is_ok(),
        "topic was deleted with its participant: {:?}",
        name.err()
    );
    assert_eq!(name.unwrap(), topic_name);
}

/// Fields drop in declaration order, so this struct destroys the participant
/// before the entities under it.
///
/// **This one passed before the fix too**, and says so rather than pretending
/// otherwise: the failing `dds_delete`s happen inside `Drop`, which cannot
/// report anything to a test. It is regression coverage for the ordering, not
/// proof of the defect — the four tests above are the proof. Run it under ASan
/// if you want the deletes themselves checked.
#[test]
fn hostile_field_order_is_survivable() {
    struct App {
        _participant: DomainParticipant,
        _subscriber: Subscriber,
        _topic: Topic<TestMessage>,
        reader: DataReader<TestMessage>,
    }

    let topic_name = unique_topic("a1_hostile_order");
    let participant = DomainParticipant::new(0).unwrap();
    let topic = Topic::<TestMessage>::new(&participant, &topic_name).unwrap();
    let subscriber = Subscriber::new(&participant).unwrap();
    let reader = DataReader::new(&subscriber, &topic).unwrap();

    let app = App {
        _participant: participant,
        _subscriber: subscriber,
        _topic: topic,
        reader,
    };

    assert!(app.reader.take().is_ok());
    drop(app);
}

/// A subscriber built from a participant that then goes away.
#[test]
fn subscriber_outliving_its_participant_still_works() {
    let subscriber = {
        let participant = DomainParticipant::new(0).unwrap();
        Subscriber::new(&participant).unwrap()
    };

    let qos = subscriber.get_qos();
    assert!(
        qos.is_ok(),
        "subscriber was deleted with its participant: {:?}",
        qos.err()
    );
}
