use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, TestMessage};

#[test]
fn waitset_guard_entities_and_ack_paths_work() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("advanced_waitset"))
        .unwrap();
    let qos = QosBuilder::new().reliable().keep_last(8).build().unwrap();
    let writer = publisher.create_writer_with_qos(&topic, &qos).unwrap();
    let reader = subscriber.create_reader_with_qos(&topic, &qos).unwrap();
    let waitset = WaitSet::new(participant.entity()).unwrap();
    let read_condition = ReadCondition::not_read(reader.entity()).unwrap();
    let guard = GuardCondition::new(participant.entity()).unwrap();

    waitset.attach(read_condition.entity(), 101).unwrap();
    waitset.attach(guard.entity(), 202).unwrap();

    let attached = waitset.get_entities().unwrap();
    assert!(attached.contains(&read_condition.entity()));
    assert!(attached.contains(&guard.entity()));

    short_delay();
    writer.write(&TestMessage::new(1, 1, "ack-me")).unwrap();
    writer.wait_for_acks(1_000_000_000).unwrap();
    assert!(!reader.take().unwrap().is_empty());

    guard.set_triggered(true).unwrap();
    let guard_wakeups = waitset.wait(1_000_000_000).unwrap();
    assert!(guard_wakeups.contains(&202));
}
