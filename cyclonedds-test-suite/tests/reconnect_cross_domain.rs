use cyclonedds::{DdsTypeDerive, DomainParticipant, Publisher, Subscriber};

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct ReconnectSample {
    id: i32,
}

#[test]
fn participant_recreate_allows_rediscovery() {
    let participant1 = DomainParticipant::new(0).unwrap();
    let topic1 = participant1
        .create_topic::<ReconnectSample>("ReconnectTest")
        .unwrap();
    let publisher = Publisher::new(&participant1).unwrap();
    let _writer: cyclonedds::DataWriter<ReconnectSample> =
        cyclonedds::DataWriter::new(&publisher, &topic1).unwrap();

    // First reader discovers the writer
    let participant2 = DomainParticipant::new(0).unwrap();
    let topic2 = participant2
        .create_topic::<ReconnectSample>("ReconnectTest")
        .unwrap();
    let subscriber = Subscriber::new(&participant2).unwrap();
    let reader: cyclonedds::DataReader<ReconnectSample> =
        cyclonedds::DataReader::new(&subscriber, &topic2).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    let matched_before = reader.matched_publications().unwrap_or_default();
    assert!(
        !matched_before.is_empty(),
        "reader should see writer before drop"
    );

    // Drop and recreate participant2
    drop(participant2);

    let participant3 = DomainParticipant::new(0).unwrap();
    let topic3 = participant3
        .create_topic::<ReconnectSample>("ReconnectTest")
        .unwrap();
    let subscriber3 = Subscriber::new(&participant3).unwrap();
    let reader3: cyclonedds::DataReader<ReconnectSample> =
        cyclonedds::DataReader::new(&subscriber3, &topic3).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    let matched_after = reader3.matched_publications().unwrap_or_default();
    assert!(
        !matched_after.is_empty(),
        "reader should rediscover writer after recreate"
    );
}

#[test]
fn cross_domain_isolation_prevents_discovery() {
    let participant_domain0 = DomainParticipant::new(0).unwrap();
    let topic0 = participant_domain0
        .create_topic::<ReconnectSample>("CrossDomainTest")
        .unwrap();
    let publisher = Publisher::new(&participant_domain0).unwrap();
    let _writer: cyclonedds::DataWriter<ReconnectSample> =
        cyclonedds::DataWriter::new(&publisher, &topic0).unwrap();

    // Reader in domain 1 should NOT discover writer in domain 0
    let participant_domain1 = DomainParticipant::new(1).unwrap();
    let topic1 = participant_domain1
        .create_topic::<ReconnectSample>("CrossDomainTest")
        .unwrap();
    let subscriber = Subscriber::new(&participant_domain1).unwrap();
    let reader: cyclonedds::DataReader<ReconnectSample> =
        cyclonedds::DataReader::new(&subscriber, &topic1).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    let matched = reader.matched_publications().unwrap_or_default();
    assert!(
        matched.is_empty(),
        "reader in domain 1 should not see writer in domain 0"
    );
}
