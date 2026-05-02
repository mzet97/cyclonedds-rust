use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, TestMessage};

#[test]
fn entity_guid_returns_non_zero_for_participant() {
    let participant = DomainParticipant::new(0).unwrap();
    let guid = participant.guid().unwrap();
    // dds_guid_t has a `v` field that is a [u8; 16] array.
    assert!(
        guid.v.iter().any(|&b| b != 0),
        "participant GUID should not be all zeros"
    );
}

#[test]
fn entity_guid_returns_non_zero_for_writer_and_reader() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("guid_test"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    let writer_guid = writer.guid().unwrap();
    let reader_guid = reader.guid().unwrap();

    assert!(
        writer_guid.v.iter().any(|&b| b != 0),
        "writer GUID should not be all zeros"
    );
    assert!(
        reader_guid.v.iter().any(|&b| b != 0),
        "reader GUID should not be all zeros"
    );
    assert_ne!(
        writer_guid.v, reader_guid.v,
        "writer and reader should have different GUIDs"
    );
}

#[test]
fn entity_status_returns_some_publication_matched_for_writer() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("status_test"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let _reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let status = writer.status().unwrap();
    // At least publication_matched should be present after a reader is created.
    assert!(
        status.publication_matched.is_some(),
        "writer should have publication_matched status"
    );
    let pm = status.publication_matched.unwrap();
    assert_eq!(
        pm.current_count, 1,
        "writer should see one matched subscription"
    );
}

#[test]
fn entity_status_returns_some_subscription_matched_for_reader() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("status_test2"))
        .unwrap();
    let _writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let status = reader.status().unwrap();
    assert!(
        status.subscription_matched.is_some(),
        "reader should have subscription_matched status"
    );
    let sm = status.subscription_matched.unwrap();
    assert_eq!(
        sm.current_count, 1,
        "reader should see one matched publication"
    );
}
