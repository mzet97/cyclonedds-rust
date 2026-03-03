//! Integration tests for cyclonedds-rust

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_participant_creation() {
    use cyclonedds::DomainParticipant;

    let participant = DomainParticipant::new(0);
    assert!(participant.is_ok());
}

#[test]
fn test_topic_creation() {
    use cyclonedds::{DomainParticipant, Topic};

    let participant = DomainParticipant::new(0).unwrap();
    let topic: Result<Topic<u8>, _> = participant.create_topic("test_topic");
    assert!(topic.is_ok());
}

#[test]
fn test_publisher_creation() {
    use cyclonedds::{DomainParticipant, Publisher};

    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.as_entity());
    assert!(publisher.is_ok());
}

#[test]
fn test_subscriber_creation() {
    use cyclonedds::{DomainParticipant, Subscriber};

    let participant = DomainParticipant::new(0).unwrap();
    let subscriber = Subscriber::new(participant.as_entity());
    assert!(subscriber.is_ok());
}
