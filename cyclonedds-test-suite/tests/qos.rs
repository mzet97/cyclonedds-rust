use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, TestMessage};

#[test]
fn qos_getters_roundtrip_real_cyclonedds_state() {
    let qos = QosBuilder::new()
        .reliability(Reliability::Reliable, 123_456)
        .durability(Durability::TransientLocal)
        .history(History::KeepLast(8))
        .deadline(10_000)
        .lifespan(20_000)
        .latency_budget(30_000)
        .ownership(Ownership::Exclusive)
        .ownership_strength(9)
        .liveliness(Liveliness::ManualByParticipant, 40_000)
        .destination_order(DestinationOrder::BySourceTimestamp)
        .writer_data_lifecycle(false)
        .transport_priority(77)
        .entity_name("rust-qos-writer")
        .userdata(vec![1, 2, 3])
        .topicdata(vec![4, 5])
        .groupdata(vec![6, 7, 8])
        .presentation(PresentationAccessScope::Group, true, false)
        .durability_service(History::KeepLast(3), 32, 16, 8)
        .ignore_local(IgnoreLocalKind::Participant)
        .writer_batching(true)
        .reader_data_lifecycle(55_000, 66_000)
        .build()
        .unwrap();

    assert_eq!(
        qos.reliability().unwrap(),
        Some((Reliability::Reliable, 123_456))
    );
    assert_eq!(qos.durability().unwrap(), Some(Durability::TransientLocal));
    assert_eq!(qos.history().unwrap(), Some(History::KeepLast(8)));
    assert_eq!(qos.deadline().unwrap(), Some(10_000));
    assert_eq!(qos.lifespan().unwrap(), Some(20_000));
    assert_eq!(qos.latency_budget().unwrap(), Some(30_000));
    assert_eq!(qos.ownership().unwrap(), Some(Ownership::Exclusive));
    assert_eq!(qos.ownership_strength().unwrap(), Some(9));
    assert_eq!(
        qos.liveliness().unwrap(),
        Some((Liveliness::ManualByParticipant, 40_000))
    );
    assert_eq!(
        qos.destination_order().unwrap(),
        Some(DestinationOrder::BySourceTimestamp)
    );
    assert_eq!(qos.writer_data_lifecycle().unwrap(), Some(false));
    assert_eq!(qos.transport_priority().unwrap(), Some(77));
    assert_eq!(
        qos.entity_name().unwrap().as_deref(),
        Some("rust-qos-writer")
    );
    assert_eq!(qos.userdata().unwrap(), Some(vec![1, 2, 3]));
    assert_eq!(qos.topicdata().unwrap(), Some(vec![4, 5]));
    assert_eq!(qos.groupdata().unwrap(), Some(vec![6, 7, 8]));

    let presentation = qos.presentation().unwrap().unwrap();
    assert_eq!(presentation.access_scope, PresentationAccessScope::Group);
    assert!(presentation.coherent_access);
    assert!(!presentation.ordered_access);

    let durability_service = qos.durability_service().unwrap().unwrap();
    assert_eq!(durability_service.history, History::KeepLast(3));
    assert_eq!(durability_service.max_samples, 32);
    assert_eq!(durability_service.max_instances, 16);
    assert_eq!(durability_service.max_samples_per_instance, 8);

    assert_eq!(
        qos.ignore_local().unwrap(),
        Some(IgnoreLocalKind::Participant)
    );
    assert_eq!(qos.writer_batching().unwrap(), Some(true));

    let reader_lifecycle = qos.reader_data_lifecycle().unwrap().unwrap();
    assert_eq!(reader_lifecycle.autopurge_nowriter_delay, 55_000);
    assert_eq!(reader_lifecycle.autopurge_disposed_delay, 66_000);
}

#[test]
fn qos_can_be_applied_to_real_writer_and_reader() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("qos_apply"))
        .unwrap();

    let writer_qos = QosBuilder::new()
        .reliable()
        .keep_last(4)
        .transient_local()
        .build()
        .unwrap();
    let reader_qos = QosBuilder::new()
        .reliable()
        .keep_last(4)
        .transient_local()
        .build()
        .unwrap();

    let writer = publisher
        .create_writer_with_qos(&topic, &writer_qos)
        .unwrap();
    let reader = subscriber
        .create_reader_with_qos(&topic, &reader_qos)
        .unwrap();

    short_delay();
    writer
        .write(&TestMessage::new(9, 900, "qos-applied"))
        .unwrap();
    short_delay();

    let data = reader.take().unwrap();
    assert!(!data.is_empty());
    assert_eq!(data[0].id, 9);
}

#[test]
fn type_consistency_and_data_representation_roundtrip() {
    let qos = Qos::builder()
        .type_consistency(TypeConsistencyPolicy {
            kind: TypeConsistency::AllowTypeCoercion,
            ignore_sequence_bounds: true,
            ignore_string_bounds: false,
            ignore_member_names: true,
            prevent_type_widening: false,
            force_type_validation: true,
        })
        .data_representation(vec![DataRepresentation::Xcdr1, DataRepresentation::Xcdr2])
        .build()
        .unwrap();

    if let Some(tc) = qos.type_consistency().unwrap() {
        assert_eq!(tc.kind, TypeConsistency::AllowTypeCoercion);
        assert!(tc.ignore_sequence_bounds);
        assert!(!tc.ignore_string_bounds);
        assert!(tc.ignore_member_names);
        assert!(!tc.prevent_type_widening);
        assert!(tc.force_type_validation);
    }

    if let Some(dr) = qos.data_representation().unwrap() {
        assert_eq!(
            dr,
            vec![DataRepresentation::Xcdr1, DataRepresentation::Xcdr2]
        );
    }
}

#[test]
fn partition_resource_limits_time_filter_and_psmx_roundtrip() {
    let qos = Qos::builder()
        .partition("alpha")
        .resource_limits(11, 22, 33)
        .time_based_filter(44_000)
        .psmx_instances(vec!["psmx-a".to_string(), "psmx-b".to_string()])
        .build()
        .unwrap();

    assert_eq!(qos.partition().unwrap().unwrap(), vec!["alpha".to_string()]);
    let limits = qos.resource_limits().unwrap().unwrap();
    assert_eq!(limits.max_samples, 11);
    assert_eq!(limits.max_instances, 22);
    assert_eq!(limits.max_samples_per_instance, 33);
    let tbf = qos.time_based_filter().unwrap().unwrap();
    assert_eq!(tbf.minimum_separation, 44_000);
    let psmx = qos.psmx_instances().unwrap().unwrap();
    assert_eq!(psmx, vec!["psmx-a".to_string(), "psmx-b".to_string()]);
}

#[test]
fn property_and_binary_property_roundtrip() {
    let qos = Qos::builder()
        .property("plain", "value")
        .property_propagate("plain-prop", "value-2", true)
        .binary_property("bin", vec![1, 2, 3])
        .binary_property_propagate("bin-prop", vec![4, 5], true)
        .build()
        .unwrap();

    assert_eq!(qos.property("plain").unwrap().unwrap(), "value");
    let propagated = qos.property_propagate("plain-prop").unwrap().unwrap();
    assert_eq!(propagated.0, "value-2");
    assert!(propagated.1);
    let prop_names = qos.property_names().unwrap().unwrap();
    assert!(prop_names.contains(&"plain".to_string()));
    assert!(prop_names.contains(&"plain-prop".to_string()));

    assert_eq!(qos.binary_property("bin").unwrap().unwrap(), vec![1, 2, 3]);
    let binary_propagated = qos.binary_property_propagate("bin-prop").unwrap().unwrap();
    assert_eq!(binary_propagated.0, vec![4, 5]);
    assert!(binary_propagated.1);
    let bprop_names = qos.binary_property_names().unwrap().unwrap();
    assert!(bprop_names.contains(&"bin".to_string()));
    assert!(bprop_names.contains(&"bin-prop".to_string()));
}

#[test]
fn qos_mutating_property_operations_work() {
    let mut qos = Qos::create().unwrap();
    qos.set_property("plain", "value").unwrap();
    qos.set_property_propagate("plain-prop", "value-2", true)
        .unwrap();
    qos.set_binary_property("bin", &[1, 2, 3]).unwrap();
    qos.set_binary_property_propagate("bin-prop", &[4, 5], true)
        .unwrap();

    assert_eq!(qos.property("plain").unwrap().unwrap(), "value");
    assert_eq!(
        qos.property_propagate("plain-prop").unwrap().unwrap(),
        ("value-2".to_string(), true)
    );
    assert_eq!(qos.binary_property("bin").unwrap().unwrap(), vec![1, 2, 3]);
    assert_eq!(
        qos.binary_property_propagate("bin-prop").unwrap().unwrap(),
        (vec![4, 5], true)
    );

    qos.unset_property("plain").unwrap();
    qos.unset_binary_property("bin").unwrap();
    assert_eq!(qos.property("plain").unwrap(), None);
    assert_eq!(qos.binary_property("bin").unwrap(), None);
}
