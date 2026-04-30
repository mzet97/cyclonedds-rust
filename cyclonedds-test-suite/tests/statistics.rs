use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, TestMessage};

#[test]
fn statistics_can_be_created_refreshed_and_queried() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("writer_statistics"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();

    short_delay();
    for idx in 0..3 {
        writer
            .write(&TestMessage::new(idx, idx * 10, "stats"))
            .unwrap();
    }

    let mut statistics = writer.create_statistics().unwrap();
    statistics.refresh().unwrap();

    assert_eq!(statistics.entity(), writer.entity());
    assert!(
        !statistics.is_empty(),
        "expected writer statistics to contain entries"
    );
    assert!(
        statistics.timestamp() > 0,
        "expected statistics refresh to populate timestamp"
    );

    let first = statistics
        .entries()
        .next()
        .expect("expected at least one statistic entry");
    assert!(!first.name().is_empty(), "expected statistic entry name");

    let looked_up = statistics
        .lookup(first.name())
        .unwrap()
        .expect("expected lookup of first statistic to succeed");
    assert_eq!(looked_up.name(), first.name());

    match looked_up.value() {
        StatisticValue::Uint32(_) | StatisticValue::Uint64(_) | StatisticValue::LengthTime(_) => {}
    }
}
