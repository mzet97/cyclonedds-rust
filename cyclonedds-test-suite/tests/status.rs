use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for, TestMessage};
use std::time::Duration;

#[test]
fn entity_relationships_and_status_masks_work() {
    let publication_matched_status =
        1u32 << cyclonedds_sys::dds_status_id_DDS_PUBLICATION_MATCHED_STATUS_ID;
    let subscription_matched_status =
        1u32 << cyclonedds_sys::dds_status_id_DDS_SUBSCRIPTION_MATCHED_STATUS_ID;
    let data_available_status = 1u32 << cyclonedds_sys::dds_status_id_DDS_DATA_AVAILABLE_STATUS_ID;
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("status_entities");
    let topic = participant
        .create_topic::<TestMessage>(&topic_name)
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    assert_eq!(topic.get_parent().unwrap(), participant.entity());
    assert_eq!(writer.get_parent().unwrap(), publisher.entity());
    assert_eq!(reader.get_parent().unwrap(), subscriber.entity());
    assert_eq!(topic.get_domain_id().unwrap(), 0);
    assert_eq!(topic.get_type_name().unwrap(), TestMessage::type_name());

    let children = participant.get_children().unwrap();
    assert!(children.contains(&publisher.entity()));
    assert!(children.contains(&subscriber.entity()));
    assert!(children.contains(&topic.entity()));

    writer.set_status_mask(publication_matched_status).unwrap();
    reader
        .set_status_mask(subscription_matched_status | data_available_status)
        .unwrap();
    assert_eq!(
        writer.get_status_mask().unwrap() & publication_matched_status,
        publication_matched_status
    );
    assert_eq!(
        reader.get_status_mask().unwrap() & subscription_matched_status,
        subscription_matched_status
    );

    assert!(wait_for(Duration::from_secs(2), || !writer
        .matched_subscriptions()
        .unwrap()
        .is_empty()));
    assert!(wait_for(Duration::from_secs(2), || !reader
        .matched_publications()
        .unwrap()
        .is_empty()));

    short_delay();
    writer.write(&TestMessage::new(3, 30, "status")).unwrap();
    assert!(wait_for(Duration::from_secs(2), || reader
        .get_status_changes()
        .unwrap()
        & data_available_status
        != 0));

    let status = reader.read_status(data_available_status).unwrap();
    assert!(status & data_available_status != 0);
    assert!(!reader.take().unwrap().is_empty());
}
