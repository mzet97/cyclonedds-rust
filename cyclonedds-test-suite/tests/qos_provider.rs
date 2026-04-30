use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for, TestMessage};
use std::time::Duration;

const QOS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<dds xmlns="http://www.omg.org/dds/"
     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
     xsi:schemaLocation="file://DDS_QoSProfile.xsd">
  <qos_library name="rustlib">
    <qos_profile name="profile">
      <domain_participant_qos>
        <user_data>
          <value>qos-provider</value>
        </user_data>
      </domain_participant_qos>
      <topic_qos name="provider_topic">
        <history>
          <kind>KEEP_LAST_HISTORY_QOS</kind>
          <depth>6</depth>
        </history>
      </topic_qos>
      <datawriter_qos>
        <reliability>
          <kind>RELIABLE_RELIABILITY_QOS</kind>
          <max_blocking_time>
            <sec>0</sec>
            <nanosec>5000000</nanosec>
          </max_blocking_time>
        </reliability>
        <durability>
          <kind>TRANSIENT_LOCAL_DURABILITY_QOS</kind>
        </durability>
        <history>
          <kind>KEEP_LAST_HISTORY_QOS</kind>
          <depth>4</depth>
        </history>
      </datawriter_qos>
      <datareader_qos>
        <reliability>
          <kind>RELIABLE_RELIABILITY_QOS</kind>
          <max_blocking_time>
            <sec>0</sec>
            <nanosec>5000000</nanosec>
          </max_blocking_time>
        </reliability>
        <durability>
          <kind>TRANSIENT_LOCAL_DURABILITY_QOS</kind>
        </durability>
        <history>
          <kind>KEEP_LAST_HISTORY_QOS</kind>
          <depth>4</depth>
        </history>
      </datareader_qos>
    </qos_profile>
  </qos_library>
</dds>
"#;

#[test]
fn qos_provider_can_load_inline_profiles_and_apply_them() {
    let provider = QosProvider::new(QOS_XML).unwrap();

    let participant_qos = provider
        .get_qos(QosKind::Participant, "rustlib::profile")
        .unwrap();
    let participant_userdata = participant_qos.userdata().unwrap().unwrap();
    assert!(!participant_userdata.is_empty());

    let participant = DomainParticipant::with_qos(0, Some(&participant_qos)).unwrap();
    let applied_participant_qos = participant.get_qos().unwrap();
    assert_eq!(
        applied_participant_qos.userdata().unwrap(),
        Some(participant_userdata)
    );

    let topic_qos = provider
        .get_qos(QosKind::Topic, "rustlib::profile::provider_topic")
        .unwrap();
    assert_eq!(topic_qos.history().unwrap(), Some(History::KeepLast(6)));

    let topic = participant
        .create_topic_with_qos::<TestMessage>(&unique_topic("qos_provider"), &topic_qos)
        .unwrap();
    let applied_topic_qos = topic.get_qos().unwrap();
    assert_eq!(
        applied_topic_qos.history().unwrap(),
        Some(History::KeepLast(6))
    );

    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();

    let writer_qos = provider
        .get_qos(QosKind::Writer, "rustlib::profile")
        .unwrap();
    let reader_qos = provider
        .get_qos(QosKind::Reader, "rustlib::profile")
        .unwrap();

    assert_eq!(
        writer_qos.reliability().unwrap(),
        Some((Reliability::Reliable, 5_000_000))
    );
    assert_eq!(
        writer_qos.durability().unwrap(),
        Some(Durability::TransientLocal)
    );
    assert_eq!(writer_qos.history().unwrap(), Some(History::KeepLast(4)));

    let writer = publisher
        .create_writer_with_qos(&topic, &writer_qos)
        .unwrap();
    let reader = subscriber
        .create_reader_with_qos(&topic, &reader_qos)
        .unwrap();

    short_delay();
    writer
        .write(&TestMessage::new(42, 420, "provider-qos"))
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));
    assert!(!reader.take().unwrap().is_empty());
}
