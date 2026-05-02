use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for, TestMessage};
use std::time::Duration;

#[test]
fn write_loan_async_publishes_sample() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let participant = DomainParticipant::new(0).unwrap();
        let publisher = participant.create_publisher().unwrap();
        let subscriber = participant.create_subscriber().unwrap();
        let topic = participant
            .create_topic::<TestMessage>(&unique_topic("loan_async"))
            .unwrap();
        let writer = publisher.create_writer(&topic).unwrap();
        let reader = subscriber.create_reader(&topic).unwrap();

        short_delay();

        writer
            .write_loan_async(|sample| {
                sample.id = 42;
                sample.value = 999;
            })
            .await
            .unwrap();

        assert!(wait_for(Duration::from_secs(2), || !reader
            .read()
            .unwrap()
            .is_empty()));

        let data = reader.take().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].id, 42);
        assert_eq!(data[0].value, 999);
    });
}
