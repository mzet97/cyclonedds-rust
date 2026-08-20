use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
struct CustomSchemaSample {
    telemetry_code: i32,
    voltage: f64,
    label: String,
}

fn custom_dynamic_type(participant: &DomainParticipant) -> DynamicType {
    let string_type = DynamicTypeBuilder::unbounded_string8()
        .build(participant)
        .unwrap();
    let mut dynamic_type = DynamicTypeBuilder::structure(CustomSchemaSample::type_name())
        .build(participant)
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("telemetry_code", DynamicPrimitiveKind::Int32).id(31),
        )
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("voltage", DynamicPrimitiveKind::Float64).id(7))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::new("label", string_type.as_spec()).id(19))
        .unwrap();
    dynamic_type
}

fn custom_data(dynamic_type: &DynamicType) -> DynamicData {
    let mut data = DynamicData::new(dynamic_type.schema());
    data.set_i32("telemetry_code", 41_337).unwrap();
    data.set_f64("voltage", 12.75).unwrap();
    data.set_string("label", "schema-name-preserved").unwrap();
    data
}

#[test]
fn synthetic_default_field_names_still_round_trip() {
    // Given: the descriptor-derived compatibility schema with synthetic names.
    let participant = DomainParticipant::new(0).unwrap();
    let type_name = unique_topic("t804_synthetic_type");
    let mut dynamic_type = DynamicTypeBuilder::structure(type_name.clone())
        .add_field(
            "field_0",
            DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
        )
        .build(&participant)
        .unwrap();
    let type_info = dynamic_type.register().unwrap();
    let discovered = discover_type_from_type_info(&participant, &type_info, &type_name, 0).unwrap();
    let mut data = DynamicData::new(&discovered.type_schema);
    data.set_i32("field_0", 90210).unwrap();

    // When: the existing public serialization path performs a round-trip.
    let bytes = dynamic_data_to_cdr(&data, &discovered.topic_descriptor).unwrap();
    let decoded = cdr_to_dynamic_data(
        &bytes,
        &discovered.type_schema,
        &discovered.topic_descriptor,
    )
    .unwrap();

    // Then: compatibility naming and the exact non-zero value remain intact.
    assert_eq!(decoded.get_i32("field_0").unwrap(), 90210);
}

#[test]
fn custom_schema_names_round_trip_through_native_serialization() {
    // Given: a builder schema with custom names and non-sequential member ids.
    let participant = DomainParticipant::new(0).unwrap();
    let mut dynamic_type = custom_dynamic_type(&participant);
    let data = custom_data(&dynamic_type);
    let descriptor = dynamic_type
        .register_topic_descriptor(&participant, FindScope::Global, 0)
        .unwrap();

    // When: public serialization and decode use the same custom schema.
    let bytes = dynamic_data_to_cdr(&data, &descriptor).unwrap();
    let decoded = cdr_to_dynamic_data(&bytes, dynamic_type.schema(), &descriptor).unwrap();

    // Then: names and distinct values survive descriptor traversal exactly.
    assert_eq!(decoded.field_names(), data.field_names());
    assert_eq!(decoded.get_i32("telemetry_code").unwrap(), 41_337);
    assert_eq!(decoded.get_f64("voltage").unwrap(), 12.75);
    assert_eq!(
        decoded.get_string("label").unwrap(),
        "schema-name-preserved"
    );
}

#[test]
fn dynamic_publish_reaches_a_real_typed_reader_with_custom_values() {
    // Given: a real reader is matched before a dynamic writer publishes the same layout.
    let participant = DomainParticipant::new(0).unwrap();
    let topic_name = unique_topic("t804_custom_publish");
    let topic = participant
        .create_topic::<CustomSchemaSample>(&topic_name)
        .unwrap();
    let subscriber = Subscriber::new(&participant).unwrap();
    let reader = DataReader::new(&subscriber, &topic).unwrap();
    let mut dynamic_type = custom_dynamic_type(&participant);
    let data = custom_data(&dynamic_type);
    short_delay();

    // When: the public dynamic publication crosses CycloneDDS and the reader takes it.
    participant
        .dynamic_publish(&topic_name, &mut dynamic_type, &data)
        .unwrap();
    let mut received = Vec::new();
    assert!(wait_for(Duration::from_secs(5), || {
        received = reader.take().unwrap();
        !received.is_empty()
    }));

    // Then: the real DDS reader observes the exact custom-field values.
    assert_eq!(
        received.last().unwrap(),
        &CustomSchemaSample {
            telemetry_code: 41_337,
            voltage: 12.75,
            label: "schema-name-preserved".to_string(),
        }
    );
}

#[test]
fn custom_schema_rejects_missing_and_extra_names() {
    // Given: valid custom data mutated through the raw-safe value boundary.
    let participant = DomainParticipant::new(0).unwrap();
    let mut dynamic_type = custom_dynamic_type(&participant);
    let descriptor = dynamic_type
        .register_topic_descriptor(&participant, FindScope::Global, 0)
        .unwrap();
    let mut missing = custom_data(&dynamic_type);
    let mut extra = custom_data(&dynamic_type);
    if let DynamicValue::Struct(fields) = missing.value_mut() {
        fields.remove("voltage");
    }
    if let DynamicValue::Struct(fields) = extra.value_mut() {
        fields.insert("rogue_name".to_string(), DynamicValue::Int32(88));
    }

    // When: malformed names cross the public serialization boundary.
    let missing_result = dynamic_data_to_cdr(&missing, &descriptor);
    let extra_result = dynamic_data_to_cdr(&extra, &descriptor);

    // Then: neither omission nor surplus is silently defaulted or ignored.
    assert!(matches!(missing_result, Err(DdsError::BadParameter(_))));
    assert!(matches!(extra_result, Err(DdsError::BadParameter(_))));
}

#[test]
fn dynamic_publish_rejects_data_from_a_different_member_schema() {
    // Given: data whose names and values match, but whose member identity differs.
    let participant = DomainParticipant::new(0).unwrap();
    let mut dynamic_type = custom_dynamic_type(&participant);
    let mut incompatible_schema = dynamic_type.schema().clone();
    if let DynamicTypeSchema::Struct { fields, .. } = &mut incompatible_schema {
        fields[0].member_id = 99;
    }
    let mut data = DynamicData::new(&incompatible_schema);
    data.set_i32("telemetry_code", 41_337).unwrap();
    data.set_f64("voltage", 12.75).unwrap();
    data.set_string("label", "schema-name-preserved").unwrap();

    // When: the mismatched data crosses the participant publication boundary.
    let result = participant.dynamic_publish(
        &unique_topic("t804_mismatched_schema"),
        &mut dynamic_type,
        &data,
    );

    // Then: publication rejects it before native traversal can reinterpret it.
    assert!(matches!(result, Err(DdsError::BadParameter(_))));
}
