use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for, TestMessage};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[test]
fn listener_callbacks_fire_with_real_entities() {
    let publication_matches = Arc::new(AtomicUsize::new(0));
    let subscription_matches = Arc::new(AtomicUsize::new(0));
    let data_available = Arc::new(AtomicUsize::new(0));

    let writer_listener = {
        let publication_matches = publication_matches.clone();
        Listener::builder()
            .on_publication_matched(move |_, status| {
                if status.current_count > 0 {
                    publication_matches.fetch_add(1, Ordering::Relaxed);
                }
            })
            .build()
            .unwrap()
    };

    let reader_listener = {
        let subscription_matches = subscription_matches.clone();
        let data_available = data_available.clone();
        Listener::builder()
            .on_subscription_matched(move |_, status| {
                if status.current_count > 0 {
                    subscription_matches.fetch_add(1, Ordering::Relaxed);
                }
            })
            .on_data_available(move |_| {
                data_available.fetch_add(1, Ordering::Relaxed);
            })
            .build()
            .unwrap()
    };

    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("listener_events"))
        .unwrap();

    let writer: DataWriter<TestMessage> =
        DataWriter::with_listener(publisher.entity(), topic.entity(), &writer_listener).unwrap();
    let reader: DataReader<TestMessage> =
        DataReader::with_listener(subscriber.entity(), topic.entity(), &reader_listener).unwrap();

    assert!(wait_for(Duration::from_secs(2), || publication_matches
        .load(Ordering::Relaxed)
        > 0));
    assert!(wait_for(Duration::from_secs(2), || subscription_matches
        .load(Ordering::Relaxed)
        > 0));

    writer
        .write(&TestMessage::new(1, 100, "listener-hit"))
        .unwrap();
    short_delay();

    assert!(wait_for(Duration::from_secs(2), || data_available
        .load(Ordering::Relaxed)
        > 0));
    assert!(!reader.take().unwrap().is_empty());
}
