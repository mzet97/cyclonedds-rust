use cyclonedds::{DomainParticipant, TopicFilterExt};
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for, TestMessage};
use std::time::Duration;

#[test]
fn topic_filter_replacement_then_clear_allows_pubsub() {
    let participant = DomainParticipant::new(232).unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("topic_filter_lifecycle"))
        .unwrap();

    unsafe {
        topic
            .set_filter(|sample: &TestMessage| sample.value > 100)
            .unwrap();
    }
    {
        let publisher = participant.create_publisher().unwrap();
        let subscriber = participant.create_subscriber().unwrap();
        let writer = publisher.create_writer(&topic).unwrap();
        let reader = subscriber.create_reader(&topic).unwrap();
        short_delay();
        writer
            .write(&TestMessage::new(1, 42, "blocked-first"))
            .unwrap();
        writer
            .write(&TestMessage::new(2, 142, "allowed-first"))
            .unwrap();
        assert!(wait_for(Duration::from_secs(2), || reader
            .read()
            .unwrap_or_default()
            .iter()
            .any(|sample| sample.id == 2)));
        assert!(reader
            .read()
            .unwrap_or_default()
            .iter()
            .all(|sample| sample.id != 1));
    }

    unsafe {
        topic
            .set_filter(|sample: &TestMessage| sample.value < 0)
            .unwrap();
    }
    {
        let publisher = participant.create_publisher().unwrap();
        let subscriber = participant.create_subscriber().unwrap();
        let writer = publisher.create_writer(&topic).unwrap();
        let reader = subscriber.create_reader(&topic).unwrap();
        short_delay();
        writer
            .write(&TestMessage::new(3, 142, "blocked-replacement"))
            .unwrap();
        writer
            .write(&TestMessage::new(4, -1, "allowed-replacement"))
            .unwrap();
        assert!(wait_for(Duration::from_secs(2), || reader
            .read()
            .unwrap_or_default()
            .iter()
            .any(|sample| sample.id == 4)));
        assert!(reader
            .read()
            .unwrap_or_default()
            .iter()
            .all(|sample| sample.id != 3));
    }

    unsafe {
        topic.clear_filter().unwrap();
    }
    {
        let publisher = participant.create_publisher().unwrap();
        let subscriber = participant.create_subscriber().unwrap();
        let writer = publisher.create_writer(&topic).unwrap();
        let reader = subscriber.create_reader(&topic).unwrap();
        short_delay();
        writer
            .write(&TestMessage::new(5, 42, "after-clear"))
            .unwrap();
        assert!(
            wait_for(Duration::from_secs(2), || reader
                .read()
                .unwrap_or_default()
                .iter()
                .any(|sample| sample.id == 5)),
            "sample published after clear_filter was not delivered"
        );
    }
}

#[test]
fn rejected_write_loans_are_returned_to_the_writer_pool() {
    let participant = DomainParticipant::new(231).unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("topic_filter_write_loan"))
        .unwrap();
    unsafe {
        topic
            .set_filter(|sample: &TestMessage| sample.value >= 0)
            .unwrap();
    }
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();
    short_delay();

    for id in 0..1_000 {
        let mut loan = writer.request_loan().unwrap();
        // SAFETY: TestMessage is valid for every bit pattern produced here; all
        // fields are initialized before the loan is submitted to CycloneDDS.
        let sample = unsafe { loan.get_mut() };
        *sample = TestMessage::new(id, -1, "rejected");
        cyclonedds::WriteLoan::write(loan).unwrap();
    }

    let mut accepted = writer.request_loan().unwrap();
    // SAFETY: as above, assigning the complete TestMessage establishes every
    // Rust and DDS wire invariant before the sample is published.
    let sample = unsafe { accepted.get_mut() };
    *sample = TestMessage::new(1_001, 1, "accepted");
    cyclonedds::WriteLoan::write(accepted).unwrap();

    assert!(wait_for(Duration::from_secs(2), || reader
        .read()
        .unwrap_or_default()
        .iter()
        .any(|sample| sample.id == 1_001)));
}
