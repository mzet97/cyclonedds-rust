use cyclonedds::*;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{}_{}_{}", prefix, std::process::id(), nanos)
}

// ── Test types (manual DdsType impl) ──

#[repr(C)]
struct HelloWorld {
    id: i32,
    message: [u8; 256],
}

impl DdsType for HelloWorld {
    fn type_name() -> &'static str {
        "HelloWorld"
    }
    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 0));
        ops.extend(adr_bst(4, 256));
        ops
    }
}

#[repr(C)]
#[derive(Debug)]
struct KeyValue {
    key: i32,
    value: i32,
}

impl DdsType for KeyValue {
    fn type_name() -> &'static str {
        "KeyValue"
    }
    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr_key(TYPE_4BY | OP_FLAG_SGN, 0));
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 4));
        ops
    }
    fn key_count() -> usize {
        1
    }
    fn keys() -> Vec<KeyDescriptor> {
        vec![KeyDescriptor {
            name: "key".into(),
            ops_path: vec![0],
        }]
    }
}

// ── Test type with derive macro ──

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct SensorReading {
    #[key]
    sensor_id: u32,
    temperature: f32,
    humidity: f32,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct StringMessage {
    id: i32,
    text: DdsString,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct SequenceMessage {
    #[key]
    id: i32,
    values: DdsSequence<i32>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct Point2D {
    x: f64,
    y: f64,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct NestedPointMessage {
    #[key]
    id: i32,
    point: Point2D,
}

#[repr(i32)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, DdsEnumDerive)]
enum SimpleState {
    Idle = 0,
    Running = 1,
    Stopped = 2,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct EnumMessage {
    #[key]
    id: i32,
    #[dds_enum]
    state: SimpleState,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct SequenceEnumMessage {
    #[key]
    id: i32,
    #[dds_enum]
    states: DdsSequence<SimpleState>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct BoundedSequenceEnumMessage {
    #[key]
    id: i32,
    #[dds_enum]
    states: DdsBoundedSequence<SimpleState, 4>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct ArrayStringMessage {
    #[key]
    id: i32,
    names: [DdsString; 3],
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct ArrayEnumMessage {
    #[key]
    id: i32,
    #[dds_enum]
    states: [SimpleState; 3],
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct DirectStringMessage {
    #[key]
    id: i32,
    text: String,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct DirectVecMessage {
    #[key]
    id: i32,
    values: Vec<i32>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct DirectVecStringMessage {
    #[key]
    id: i32,
    values: Vec<String>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct DirectVecEnumMessage {
    #[key]
    id: i32,
    #[dds_enum]
    states: Vec<SimpleState>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct DirectVecStructMessage {
    #[key]
    id: i32,
    points: Vec<Point2D>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct MultiArrayMessage {
    #[key]
    id: i32,
    matrix: [[i32; 3]; 2],
    #[dds_enum]
    states: [[SimpleState; 2]; 2],
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct NestedSequencesMessage {
    #[key]
    id: i32,
    matrix: DdsSequence<DdsSequence<i32>>,
    bounded_matrix: DdsBoundedSequence<DdsBoundedSequence<f64, 2>, 2>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct OptionalMessage {
    #[key]
    id: i32,
    opt_long: Option<i32>,
    opt_double: Option<f64>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct BoundedSequenceMessage {
    #[key]
    id: i32,
    values: DdsBoundedSequence<i32, 4>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct BoundedSequenceStructMessage {
    #[key]
    id: i32,
    points: DdsBoundedSequence<Point2D, 3>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct SequenceStructMessage {
    #[key]
    id: i32,
    points: DdsSequence<Point2D>,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct ArrayStructMessage {
    #[key]
    id: i32,
    points: [Point2D; 3],
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct Location {
    #[key]
    building: i32,
    #[key]
    floor: i16,
    room: i32,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct NestedKeyMessage {
    location: Location,
    description: [u8; 64],
}

// ── Basic entity creation tests ──

#[test]
fn test_participant_creation() {
    let participant = DomainParticipant::new(0);
    assert!(
        participant.is_ok(),
        "Failed to create participant: {:?}",
        participant.err()
    );
}

#[test]
fn test_topic_creation() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic: Result<Topic<HelloWorld>, _> = participant.create_topic("test_topic_creation");
    assert!(topic.is_ok(), "Failed to create topic: {:?}", topic.err());
}

#[test]
fn test_keyed_instance_lifecycle() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<KeyValue>("test_keyed_topic")
        .unwrap();
    let writer = DataWriter::<KeyValue>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<KeyValue>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let ih1 = writer
        .register_instance(&KeyValue { key: 1, value: 100 })
        .expect("register failed");
    let ih2 = writer
        .register_instance(&KeyValue { key: 2, value: 200 })
        .expect("register failed");
    assert_ne!(ih1, 0);
    assert_ne!(ih2, 0);
    assert_ne!(ih1, ih2, "Different keys must produce different handles");

    writer
        .write(&KeyValue { key: 1, value: 101 })
        .expect("write1 failed");
    writer
        .write(&KeyValue { key: 2, value: 201 })
        .expect("write2 failed");

    thread::sleep(Duration::from_millis(500));

    let samples: Vec<KeyValue> = reader.take().expect("take failed");
    assert!(
        samples.len() >= 2,
        "Expected >= 2 samples, got {}",
        samples.len()
    );

    writer
        .write_dispose(&KeyValue { key: 1, value: 999 })
        .expect("writedispose failed");
    writer
        .unregister_instance_handle(ih2)
        .expect("unregister failed");
}

#[test]
fn test_subscriber_creation() {
    let participant = DomainParticipant::new(0).unwrap();
    let subscriber = Subscriber::new(participant.entity());
    assert!(
        subscriber.is_ok(),
        "Failed to create subscriber: {:?}",
        subscriber.err()
    );
}

#[test]
fn test_writer_reader_creation() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<HelloWorld>("test_writer_reader")
        .unwrap();

    let writer = DataWriter::<HelloWorld>::new(publisher.entity(), topic.entity());
    assert!(
        writer.is_ok(),
        "Failed to create writer: {:?}",
        writer.err()
    );

    let reader = DataReader::<HelloWorld>::new(subscriber.entity(), topic.entity());
    assert!(
        reader.is_ok(),
        "Failed to create reader: {:?}",
        reader.err()
    );
}

#[test]
fn test_qos_builder() {
    let qos = QosBuilder::new()
        .reliable()
        .transient_local()
        .keep_last(10)
        .build();
    assert!(qos.is_ok(), "Failed to build QoS: {:?}", qos.err());
}

#[test]
fn test_waitset_creation() {
    let participant = DomainParticipant::new(0).unwrap();
    let waitset = WaitSet::new(participant.entity());
    assert!(
        waitset.is_ok(),
        "Failed to create WaitSet: {:?}",
        waitset.err()
    );
}

#[test]
fn test_readcondition_creation() {
    let participant = DomainParticipant::new(0).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant.create_topic::<HelloWorld>("test_rc").unwrap();
    let reader: DataReader<HelloWorld> =
        DataReader::new(subscriber.entity(), topic.entity()).unwrap();

    let cond = ReadCondition::not_read(reader.entity());
    assert!(
        cond.is_ok(),
        "Failed to create ReadCondition: {:?}",
        cond.err()
    );
}

#[test]
fn test_guardcondition() {
    let participant = DomainParticipant::new(0).unwrap();
    let guard = GuardCondition::new(participant.entity());
    assert!(
        guard.is_ok(),
        "Failed to create GuardCondition: {:?}",
        guard.err()
    );

    let guard = guard.unwrap();
    let trigger_result = guard.set_triggered(true);
    assert!(
        trigger_result.is_ok(),
        "Failed to trigger GuardCondition: {:?}",
        trigger_result.err()
    );

    // Test read/take
    let read_result = guard.read();
    assert!(
        read_result.is_ok(),
        "Failed to read GuardCondition: {:?}",
        read_result.err()
    );
    assert!(
        read_result.unwrap(),
        "GuardCondition should be triggered after read"
    );

    let take_result = guard.take();
    assert!(
        take_result.is_ok(),
        "Failed to take GuardCondition: {:?}",
        take_result.err()
    );
    assert!(
        take_result.unwrap(),
        "GuardCondition should be triggered after take"
    );

    // After take, it should be reset
    let read_after_take = guard.read();
    assert!(read_after_take.is_ok());
    assert!(
        !read_after_take.unwrap(),
        "GuardCondition should be reset after take"
    );
}

// ── End-to-end pub/sub test ──

#[test]
fn test_pub_sub_e2e() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<HelloWorld>("test_e2e_topic")
        .unwrap();

    let writer = DataWriter::<HelloWorld>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<HelloWorld>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let mut msg = HelloWorld {
        id: 42,
        message: [0u8; 256],
    };
    let text = b"Integration test!";
    msg.message[..text.len()].copy_from_slice(text);

    writer.write(&msg).expect("write failed");

    // Allow data to propagate
    thread::sleep(Duration::from_millis(500));

    let samples: Vec<HelloWorld> = reader.take().expect("take failed");
    assert!(!samples.is_empty(), "Expected at least one sample, got 0");

    let sample = &samples[0];
    assert_eq!(sample.id, 42);
    let end = sample.message.iter().position(|&b| b == 0).unwrap_or(256);
    let received_text = std::str::from_utf8(&sample.message[..end]).unwrap_or("?");
    assert_eq!(received_text, "Integration test!");
}

// ── Derive macro test ──

#[test]
fn test_derive_keyed_pub_sub() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<SensorReading>("test_derive_sensor")
        .unwrap();
    let writer = DataWriter::<SensorReading>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<SensorReading>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let ih1 = writer
        .register_instance(&SensorReading {
            sensor_id: 10,
            temperature: 23.5,
            humidity: 65.0,
        })
        .unwrap();
    let ih2 = writer
        .register_instance(&SensorReading {
            sensor_id: 20,
            temperature: 18.0,
            humidity: 72.0,
        })
        .unwrap();
    assert_ne!(
        ih1, ih2,
        "Different sensor_id keys must produce different handles"
    );

    writer
        .write(&SensorReading {
            sensor_id: 10,
            temperature: 24.0,
            humidity: 64.0,
        })
        .unwrap();
    writer
        .write(&SensorReading {
            sensor_id: 20,
            temperature: 19.0,
            humidity: 71.0,
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples: Vec<SensorReading> = reader.take().unwrap();
    assert!(
        samples.len() >= 2,
        "Expected >= 2 samples, got {}",
        samples.len()
    );
}

#[test]
fn test_string_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<StringMessage>("test_string_roundtrip")
        .unwrap();
    let writer = DataWriter::<StringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<StringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&StringMessage {
            id: 5,
            text: DdsString::new("hello from cyclonedds string").unwrap(),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(!samples.is_empty(), "Expected at least one string sample");
    assert_eq!(samples[0].id, 5);
    assert_eq!(
        samples[0].text.to_string_lossy(),
        "hello from cyclonedds string"
    );
}

#[test]
fn test_sequence_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<SequenceMessage>("test_sequence_roundtrip")
        .unwrap();
    let writer = DataWriter::<SequenceMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<SequenceMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let seq = DdsSequence::from_slice(&[10, 20, 30, 40]).unwrap();
    writer
        .write(&SequenceMessage { id: 7, values: seq })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(!samples.is_empty(), "Expected at least one sequence sample");
    assert_eq!(samples[0].id, 7);
    assert_eq!(samples[0].values.to_vec(), vec![10, 20, 30, 40]);
}

#[test]
fn test_nested_struct_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<NestedPointMessage>("test_nested_struct_roundtrip")
        .unwrap();
    let writer = DataWriter::<NestedPointMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<NestedPointMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&NestedPointMessage {
            id: 11,
            point: Point2D { x: 1.25, y: -9.5 },
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(!samples.is_empty(), "Expected at least one nested sample");
    assert_eq!(samples[0].id, 11);
    assert!((samples[0].point.x - 1.25).abs() < f64::EPSILON);
    assert!((samples[0].point.y + 9.5).abs() < f64::EPSILON);
}

#[test]
fn test_enum_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<EnumMessage>("test_enum_roundtrip")
        .unwrap();
    let writer = DataWriter::<EnumMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<EnumMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&EnumMessage {
            id: 21,
            state: SimpleState::Running,
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(!samples.is_empty(), "Expected at least one enum sample");
    assert_eq!(samples[0].id, 21);
    assert_eq!(samples[0].state, SimpleState::Running);
}

#[test]
fn test_bounded_sequence_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<BoundedSequenceMessage>("test_bounded_sequence_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<BoundedSequenceMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<BoundedSequenceMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&BoundedSequenceMessage {
            id: 31,
            values: DdsBoundedSequence::from_slice(&[2, 4, 6, 8]).unwrap(),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one bounded sequence sample"
    );
    assert_eq!(samples[0].id, 31);
    assert_eq!(samples[0].values.to_vec(), vec![2, 4, 6, 8]);
}

#[test]
fn test_sequence_struct_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<SequenceStructMessage>("test_sequence_struct_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<SequenceStructMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<SequenceStructMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&SequenceStructMessage {
            id: 41,
            points: DdsSequence::from_slice(&[
                Point2D { x: 1.0, y: 2.0 },
                Point2D { x: 3.0, y: 4.0 },
            ])
            .unwrap(),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one sequence-of-struct sample"
    );
    assert_eq!(samples[0].id, 41);
    let points = samples[0].points.to_vec();
    assert_eq!(points.len(), 2);
    assert!((points[0].x - 1.0).abs() < f64::EPSILON);
    assert!((points[0].y - 2.0).abs() < f64::EPSILON);
    assert!((points[1].x - 3.0).abs() < f64::EPSILON);
    assert!((points[1].y - 4.0).abs() < f64::EPSILON);
}

#[test]
fn test_array_struct_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<ArrayStructMessage>("test_array_struct_roundtrip")
        .unwrap();
    let writer = DataWriter::<ArrayStructMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<ArrayStructMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&ArrayStructMessage {
            id: 55,
            points: [
                Point2D { x: -1.0, y: 0.5 },
                Point2D { x: 2.5, y: 3.5 },
                Point2D { x: 8.0, y: 13.0 },
            ],
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one array-of-struct sample"
    );
    assert_eq!(samples[0].id, 55);
    assert!((samples[0].points[0].x + 1.0).abs() < f64::EPSILON);
    assert!((samples[0].points[0].y - 0.5).abs() < f64::EPSILON);
    assert!((samples[0].points[1].x - 2.5).abs() < f64::EPSILON);
    assert!((samples[0].points[1].y - 3.5).abs() < f64::EPSILON);
    assert!((samples[0].points[2].x - 8.0).abs() < f64::EPSILON);
    assert!((samples[0].points[2].y - 13.0).abs() < f64::EPSILON);
}

#[test]
fn test_bounded_sequence_struct_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<BoundedSequenceStructMessage>("test_bounded_sequence_struct_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<BoundedSequenceStructMessage>::new(publisher.entity(), topic.entity())
            .unwrap();
    let reader =
        DataReader::<BoundedSequenceStructMessage>::new(subscriber.entity(), topic.entity())
            .unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&BoundedSequenceStructMessage {
            id: 51,
            points: DdsBoundedSequence::from_slice(&[
                Point2D { x: 1.0, y: 1.5 },
                Point2D { x: 2.0, y: 2.5 },
            ])
            .unwrap(),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one bounded sequence-of-struct sample"
    );
    assert_eq!(samples[0].id, 51);
    let points = samples[0].points.to_vec();
    assert_eq!(points.len(), 2);
    assert!((points[0].x - 1.0).abs() < f64::EPSILON);
    assert!((points[0].y - 1.5).abs() < f64::EPSILON);
    assert!((points[1].x - 2.0).abs() < f64::EPSILON);
    assert!((points[1].y - 2.5).abs() < f64::EPSILON);
}

#[test]
fn test_sequence_enum_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<SequenceEnumMessage>("test_sequence_enum_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<SequenceEnumMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<SequenceEnumMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&SequenceEnumMessage {
            id: 61,
            states: DdsSequence::from_slice(&[
                SimpleState::Idle,
                SimpleState::Running,
                SimpleState::Stopped,
            ])
            .unwrap(),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one sequence-of-enum sample"
    );
    assert_eq!(samples[0].id, 61);
    assert_eq!(
        samples[0].states.to_vec(),
        vec![
            SimpleState::Idle,
            SimpleState::Running,
            SimpleState::Stopped
        ]
    );
}

#[test]
fn test_bounded_sequence_enum_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<BoundedSequenceEnumMessage>("test_bounded_sequence_enum_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<BoundedSequenceEnumMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<BoundedSequenceEnumMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&BoundedSequenceEnumMessage {
            id: 71,
            states: DdsBoundedSequence::from_slice(&[SimpleState::Running, SimpleState::Stopped])
                .unwrap(),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one bounded sequence-of-enum sample"
    );
    assert_eq!(samples[0].id, 71);
    assert_eq!(
        samples[0].states.to_vec(),
        vec![SimpleState::Running, SimpleState::Stopped]
    );
}

#[test]
fn test_array_string_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<ArrayStringMessage>("test_array_string_roundtrip")
        .unwrap();
    let writer = DataWriter::<ArrayStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<ArrayStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&ArrayStringMessage {
            id: 81,
            names: [
                DdsString::new("alpha").unwrap(),
                DdsString::new("beta").unwrap(),
                DdsString::new("gamma").unwrap(),
            ],
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one array-of-string sample"
    );
    assert_eq!(samples[0].id, 81);
    assert_eq!(samples[0].names[0].to_string_lossy(), "alpha");
    assert_eq!(samples[0].names[1].to_string_lossy(), "beta");
    assert_eq!(samples[0].names[2].to_string_lossy(), "gamma");
}

#[test]
fn test_array_enum_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<ArrayEnumMessage>("test_array_enum_roundtrip")
        .unwrap();
    let writer = DataWriter::<ArrayEnumMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<ArrayEnumMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&ArrayEnumMessage {
            id: 91,
            states: [
                SimpleState::Idle,
                SimpleState::Running,
                SimpleState::Stopped,
            ],
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one array-of-enum sample"
    );
    assert_eq!(samples[0].id, 91);
    assert_eq!(
        samples[0].states,
        [
            SimpleState::Idle,
            SimpleState::Running,
            SimpleState::Stopped
        ]
    );
}

#[test]
fn test_direct_string_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<DirectStringMessage>("test_direct_string_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&DirectStringMessage {
            id: 101,
            text: "idiomatic-string".to_string(),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one direct-string sample"
    );
    assert_eq!(samples[0].id, 101);
    assert_eq!(samples[0].text, "idiomatic-string");
}

#[test]
fn test_direct_vec_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<DirectVecMessage>("test_direct_vec_roundtrip")
        .unwrap();
    let writer = DataWriter::<DirectVecMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<DirectVecMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&DirectVecMessage {
            id: 202,
            values: vec![11, 22, 33, 44],
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one direct-vec sample"
    );
    assert_eq!(samples[0].id, 202);
    assert_eq!(samples[0].values, vec![11, 22, 33, 44]);
}

#[test]
fn test_direct_vec_string_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<DirectVecStringMessage>("test_direct_vec_string_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<DirectVecStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<DirectVecStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&DirectVecStringMessage {
            id: 303,
            values: vec!["red".to_string(), "green".to_string(), "blue".to_string()],
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one direct-vec-string sample"
    );
    assert_eq!(samples[0].id, 303);
    assert_eq!(
        samples[0].values,
        vec!["red".to_string(), "green".to_string(), "blue".to_string()]
    );
}

#[test]
fn test_direct_vec_enum_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<DirectVecEnumMessage>("test_direct_vec_enum_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<DirectVecEnumMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<DirectVecEnumMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&DirectVecEnumMessage {
            id: 404,
            states: vec![
                SimpleState::Idle,
                SimpleState::Running,
                SimpleState::Stopped,
            ],
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one direct-vec-enum sample"
    );
    assert_eq!(samples[0].id, 404);
    assert_eq!(
        samples[0].states,
        vec![
            SimpleState::Idle,
            SimpleState::Running,
            SimpleState::Stopped
        ]
    );
}

#[test]
fn test_direct_vec_struct_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<DirectVecStructMessage>("test_direct_vec_struct_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<DirectVecStructMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<DirectVecStructMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&DirectVecStructMessage {
            id: 505,
            points: vec![Point2D { x: 1.0, y: 1.5 }, Point2D { x: 2.0, y: 2.5 }],
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one direct-vec-struct sample"
    );
    assert_eq!(samples[0].id, 505);
    assert_eq!(samples[0].points.len(), 2);
    assert!((samples[0].points[0].x - 1.0).abs() < f64::EPSILON);
    assert!((samples[0].points[0].y - 1.5).abs() < f64::EPSILON);
    assert!((samples[0].points[1].x - 2.0).abs() < f64::EPSILON);
    assert!((samples[0].points[1].y - 2.5).abs() < f64::EPSILON);
}

#[test]
fn test_multi_array_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<MultiArrayMessage>("test_multi_array_roundtrip")
        .unwrap();
    let writer = DataWriter::<MultiArrayMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<MultiArrayMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&MultiArrayMessage {
            id: 505,
            matrix: [[1, 2, 3], [4, 5, 6]],
            states: [
                [SimpleState::Idle, SimpleState::Running],
                [SimpleState::Stopped, SimpleState::Idle],
            ],
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one multi-array sample"
    );
    assert_eq!(samples[0].id, 505);
    assert_eq!(samples[0].matrix, [[1, 2, 3], [4, 5, 6]]);
    assert_eq!(
        samples[0].states,
        [
            [SimpleState::Idle, SimpleState::Running],
            [SimpleState::Stopped, SimpleState::Idle],
        ]
    );
}

#[test]
fn test_nested_sequences_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<NestedSequencesMessage>("test_nested_sequences_roundtrip")
        .unwrap();
    let writer =
        DataWriter::<NestedSequencesMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<NestedSequencesMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&NestedSequencesMessage {
            id: 606,
            matrix: DdsSequence::from_slice(&[
                DdsSequence::from_slice(&[1, 2]).unwrap(),
                DdsSequence::from_slice(&[3, 4, 5]).unwrap(),
            ])
            .unwrap(),
            bounded_matrix: DdsBoundedSequence::from_slice(&[
                DdsBoundedSequence::from_slice(&[1.5, 2.5]).unwrap(),
                DdsBoundedSequence::from_slice(&[3.5]).unwrap(),
            ])
            .unwrap(),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one nested-sequences sample"
    );
    assert_eq!(samples[0].id, 606);
    let matrix = samples[0].matrix.to_vec();
    assert_eq!(matrix.len(), 2);
    assert_eq!(matrix[0].to_vec(), vec![1, 2]);
    assert_eq!(matrix[1].to_vec(), vec![3, 4, 5]);
    let bounded = samples[0].bounded_matrix.to_vec();
    assert_eq!(bounded.len(), 2);
    assert_eq!(bounded[0].to_vec(), vec![1.5, 2.5]);
    assert_eq!(bounded[1].to_vec(), vec![3.5]);
}

#[test]
fn test_optional_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<OptionalMessage>("test_optional_roundtrip")
        .unwrap();
    let writer = DataWriter::<OptionalMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<OptionalMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    writer
        .write(&OptionalMessage {
            id: 1201,
            opt_long: Some(42),
            opt_double: Some(3.5),
        })
        .unwrap();

    thread::sleep(Duration::from_millis(500));

    let samples = reader.take().unwrap();
    assert!(!samples.is_empty(), "Expected at least one optional sample");
    assert_eq!(samples[0].id, 1201);
    assert_eq!(samples[0].opt_long, Some(42));
    assert_eq!(samples[0].opt_double, Some(3.5));
}

#[test]
fn test_type_info_and_matched_endpoint_discovery() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<DirectStringMessage>("test_type_info_and_matched_endpoint_discovery")
        .unwrap();
    let writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let _ = topic.get_type_info();
    let _ = writer.get_type_info();
    let _ = reader.get_type_info();

    let sub_endpoints = writer.matched_subscription_endpoints().unwrap();
    assert!(
        !sub_endpoints.is_empty(),
        "expected at least one matched subscription endpoint"
    );
    assert!(sub_endpoints[0]
        .topic_name()
        .contains("test_type_info_and_matched_endpoint_discovery"));
    assert!(!sub_endpoints[0].type_name().is_empty());
    let _ = sub_endpoints[0].type_info();

    let pub_endpoints = reader.matched_publication_endpoints().unwrap();
    assert!(
        !pub_endpoints.is_empty(),
        "expected at least one matched publication endpoint"
    );
    assert!(pub_endpoints[0]
        .topic_name()
        .contains("test_type_info_and_matched_endpoint_discovery"));
    assert!(!pub_endpoints[0].type_name().is_empty());
    let _ = pub_endpoints[0].type_info();

    // Test get_matched_subscription_data / get_matched_publication_data
    let sub_handles = writer.matched_subscriptions().unwrap();
    assert!(!sub_handles.is_empty());
    let sub_data = writer.get_matched_subscription_data(sub_handles[0]);
    assert!(
        sub_data.is_ok(),
        "get_matched_subscription_data failed: {:?}",
        sub_data.err()
    );
    assert!(sub_data
        .unwrap()
        .topic_name()
        .contains("test_type_info_and_matched_endpoint_discovery"));

    let pub_handles = reader.matched_publications().unwrap();
    assert!(!pub_handles.is_empty());
    let pub_data = reader.get_matched_publication_data(pub_handles[0]);
    assert!(
        pub_data.is_ok(),
        "get_matched_publication_data failed: {:?}",
        pub_data.err()
    );
    assert!(pub_data
        .unwrap()
        .topic_name()
        .contains("test_type_info_and_matched_endpoint_discovery"));
}

#[test]
fn test_nested_key_roundtrip() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<NestedKeyMessage>("test_nested_key_roundtrip")
        .unwrap();
    let writer = DataWriter::<NestedKeyMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<NestedKeyMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let sample = NestedKeyMessage {
        location: Location {
            building: 7,
            floor: 3,
            room: 42,
        },
        description: {
            let mut d = [0u8; 64];
            d[..10].copy_from_slice(b"nested-key");
            d
        },
    };

    let ih = writer.register_instance(&sample).unwrap();
    assert_ne!(ih, 0);
    writer.write(&sample).unwrap();

    thread::sleep(Duration::from_millis(500));

    let lookup = writer.lookup_instance(&sample);
    assert_eq!(lookup, ih);

    let samples = reader.take().unwrap();
    assert!(
        !samples.is_empty(),
        "Expected at least one nested-key sample"
    );
    assert_eq!(samples[0].location.building, 7);
    assert_eq!(samples[0].location.floor, 3);
    assert_eq!(samples[0].location.room, 42);
}

#[test]
fn test_find_topic_and_topic_descriptor_from_type_info() {
    let participant1 = DomainParticipant::new(0).unwrap();
    let participant2 = DomainParticipant::new(0).unwrap();

    let type_name = unique_name("find_topic_dynamic_type");
    let topic_name = unique_name("find_topic_dynamic_topic");

    let mut dynamic_type = participant1
        .create_dynamic_type(DynamicTypeBuilder::structure(type_name.clone()))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(10))
        .unwrap();
    dynamic_type.set_member_key(10, true).unwrap();

    let type_info = dynamic_type.register().unwrap();
    let descriptor = participant1
        .create_topic_descriptor(&type_info, FindScope::LocalDomain, 0)
        .unwrap();
    let topic = participant1
        .create_topic_from_descriptor(&topic_name, &descriptor)
        .unwrap();

    thread::sleep(Duration::from_millis(200));

    let cloned_descriptor = participant2
        .create_topic_descriptor(&type_info, FindScope::LocalDomain, 0)
        .unwrap();
    assert!(cloned_descriptor.op_count() > 0);
    assert_eq!(cloned_descriptor.type_name(), type_name);

    let found = participant2
        .find_topic(FindScope::LocalDomain, &topic_name, Some(&type_info), 0)
        .unwrap()
        .expect("expected discovered topic");
    assert_eq!(found.get_name().unwrap(), topic_name);
    assert_eq!(
        found.get_type_name().unwrap(),
        topic.get_type_name().unwrap()
    );
    let _ = found.get_type_info().unwrap();
}

#[test]
fn test_dynamic_type_registration_and_descriptor_creation() {
    let participant = DomainParticipant::new(117).unwrap();

    let type_name = unique_name("dynamic_struct_type");
    let sub_type_name = unique_name("dynamic_sub_type");
    let enum_type_name = unique_name("dynamic_enum_type");
    let seq_type_name = unique_name("dynamic_seq_type");
    let topic_name = unique_name("dynamic_struct_topic");

    let mut sub_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(sub_type_name.clone()))
        .unwrap();
    sub_type
        .add_member(
            DynamicMemberBuilder::primitive("sub_value", DynamicPrimitiveKind::UInt16).id(1),
        )
        .unwrap();

    let mut enum_type = participant
        .create_dynamic_type(DynamicTypeBuilder::enumeration(enum_type_name.clone()))
        .unwrap();
    enum_type
        .add_enum_literal("Idle", DynamicEnumLiteralValue::NextAvailable, false)
        .unwrap();
    enum_type
        .add_enum_literal("Busy", DynamicEnumLiteralValue::Explicit(7), true)
        .unwrap();

    let sequence_type = participant
        .create_dynamic_type(DynamicTypeBuilder::sequence(
            seq_type_name.clone(),
            DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
            Some(8),
        ))
        .unwrap();

    let mut dynamic_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(type_name.clone()))
        .unwrap();
    dynamic_type
        .set_extensibility(DynamicTypeExtensibility::Appendable)
        .unwrap();
    dynamic_type
        .set_autoid(DynamicTypeAutoId::Sequential)
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(10))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::new("sub", sub_type.as_spec()).id(20))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::new("values", sequence_type.as_spec()).id(30))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::new("state", enum_type.as_spec()).id(40))
        .unwrap();
    dynamic_type.set_member_key(10, true).unwrap();
    dynamic_type.set_member_external(20, true).unwrap();
    dynamic_type.set_member_optional(30, true).unwrap();
    dynamic_type.set_member_must_understand(40, true).unwrap();

    let descriptor = dynamic_type
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(descriptor.type_name(), type_name);
    assert_eq!(descriptor.key_count(), 1);
    assert!(descriptor.op_count() > 0);
    assert!(descriptor.size() > 0);
    assert!(descriptor.align() > 0);

    let topic = participant
        .create_topic_from_descriptor(&topic_name, &descriptor)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), type_name);
    let _ = topic.get_type_info().unwrap();
}

#[test]
fn test_dynamic_union_bitmask_and_alias_registration() {
    let participant = DomainParticipant::new(118).unwrap();

    let sub_type_name = unique_name("dynamic_union_subtype");
    let union_type_name = unique_name("dynamic_union_type");
    let bitmask_type_name = unique_name("dynamic_bitmask_type");
    let alias_type_name = unique_name("dynamic_alias_type");
    let root_type_name = unique_name("dynamic_root_type");
    let topic_name = unique_name("dynamic_root_topic");

    let mut sub_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(sub_type_name.clone()))
        .unwrap();
    sub_type.set_nested(true).unwrap();
    sub_type
        .add_member(DynamicMemberBuilder::primitive(
            "sub_value",
            DynamicPrimitiveKind::UInt16,
        ))
        .unwrap();

    let mut union_type = participant
        .create_dynamic_type(DynamicTypeBuilder::union(
            union_type_name.clone(),
            DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
        ))
        .unwrap();
    union_type.set_nested(true).unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("as_i32", DynamicPrimitiveKind::Int32).labels(&[1, 2]),
        )
        .unwrap();
    union_type
        .add_member(DynamicMemberBuilder::new("as_sub", sub_type.as_spec()).labels(&[5]))
        .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("as_bool", DynamicPrimitiveKind::Boolean)
                .default_label(),
        )
        .unwrap();

    let mut bitmask_type = participant
        .create_dynamic_type(DynamicTypeBuilder::bitmask(bitmask_type_name.clone()))
        .unwrap();
    bitmask_type.set_bit_bound(16).unwrap();
    bitmask_type.add_bitmask_field("flag_a", None).unwrap();
    bitmask_type.add_bitmask_field("flag_b", Some(5)).unwrap();
    bitmask_type.add_bitmask_field("flag_c", None).unwrap();

    let string_type = participant
        .create_dynamic_type(DynamicTypeBuilder::string8(Some(32)))
        .unwrap();
    let alias_type = participant
        .create_dynamic_type(DynamicTypeBuilder::alias(
            alias_type_name.clone(),
            string_type.as_spec(),
        ))
        .unwrap();

    let mut root_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(root_type_name.clone()))
        .unwrap();
    root_type
        .set_extensibility(DynamicTypeExtensibility::Appendable)
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(10))
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("choice", union_type.as_spec()).id(20))
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("flags", bitmask_type.as_spec()).id(30))
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("name", alias_type.as_spec()).id(40))
        .unwrap();
    root_type.set_member_key(10, true).unwrap();
    root_type.set_member_external(20, true).unwrap();
    root_type.set_member_must_understand(30, true).unwrap();
    root_type.set_member_optional(40, true).unwrap();

    let descriptor = root_type
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(descriptor.type_name(), root_type_name);
    assert_eq!(descriptor.key_count(), 1);
    assert!(descriptor.op_count() > 0);

    let topic = participant
        .create_topic_from_descriptor(&topic_name, &descriptor)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), root_type_name);
    let _ = topic.get_type_info().unwrap();
}

#[test]
fn test_matched_endpoint_qos_and_keys_are_accessible() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectStringMessage>(&unique_name(
            "test_matched_endpoint_qos_and_keys_are_accessible",
        ))
        .unwrap();

    let writer_qos = Qos::builder()
        .entity_name("writer-endpoint")
        .reliable()
        .build()
        .unwrap();
    let reader_qos = Qos::builder()
        .entity_name("reader-endpoint")
        .reliable()
        .build()
        .unwrap();

    let writer = DataWriter::<DirectStringMessage>::with_qos(
        publisher.entity(),
        topic.entity(),
        Some(&writer_qos),
    )
    .unwrap();
    let reader = DataReader::<DirectStringMessage>::with_qos(
        subscriber.entity(),
        topic.entity(),
        Some(&reader_qos),
    )
    .unwrap();

    thread::sleep(Duration::from_millis(500));

    let sub_endpoints = writer.matched_subscription_endpoints().unwrap();
    assert!(!sub_endpoints.is_empty());
    let sub = &sub_endpoints[0];
    assert_ne!(sub.key().v, [0; 16]);
    assert_ne!(sub.participant_key().v, [0; 16]);
    assert_ne!(sub.participant_instance_handle(), 0);
    let sub_qos = sub.qos().unwrap().unwrap();
    assert_eq!(sub_qos.entity_name().unwrap().unwrap(), "reader-endpoint");

    let pub_endpoints = reader.matched_publication_endpoints().unwrap();
    assert!(!pub_endpoints.is_empty());
    let pub_ep = &pub_endpoints[0];
    assert_ne!(pub_ep.key().v, [0; 16]);
    assert_ne!(pub_ep.participant_key().v, [0; 16]);
    assert_ne!(pub_ep.participant_instance_handle(), 0);
    let pub_qos = pub_ep.qos().unwrap().unwrap();
    assert_eq!(pub_qos.entity_name().unwrap().unwrap(), "writer-endpoint");
}

#[test]
fn test_matched_endpoint_can_create_topic_descriptor_and_topic() {
    let participant1 = DomainParticipant::new(0).unwrap();
    let participant3 = DomainParticipant::new(0).unwrap();

    let publisher = participant1.create_publisher().unwrap();
    let subscriber = participant1.create_subscriber().unwrap();
    let topic_name = unique_name("test_matched_endpoint_can_create_topic_descriptor_and_topic");
    let topic = participant1
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();

    let writer_qos = Qos::builder()
        .entity_name("writer-match")
        .reliable()
        .build()
        .unwrap();
    let reader_qos = Qos::builder()
        .entity_name("reader-match")
        .reliable()
        .build()
        .unwrap();
    let writer = DataWriter::<DirectStringMessage>::with_qos(
        publisher.entity(),
        topic.entity(),
        Some(&writer_qos),
    )
    .unwrap();
    let _reader = DataReader::<DirectStringMessage>::with_qos(
        subscriber.entity(),
        topic.entity(),
        Some(&reader_qos),
    )
    .unwrap();

    thread::sleep(Duration::from_millis(500));

    let endpoint = writer
        .matched_subscription_endpoints()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    if let Ok(descriptor) =
        endpoint.create_topic_descriptor(participant3.entity(), FindScope::LocalDomain, 0)
    {
        assert_eq!(descriptor.type_name(), "DirectStringMessage");
        let discovered_topic = endpoint
            .create_topic(participant3.entity(), FindScope::LocalDomain, 0)
            .unwrap();
        assert_eq!(discovered_topic.get_name().unwrap(), topic_name);
        assert_eq!(
            discovered_topic.get_type_name().unwrap(),
            "DirectStringMessage"
        );
    }
}

#[test]
fn test_dynamic_base_type_and_duplicate_registration() {
    let participant = DomainParticipant::new(111).unwrap();

    let base_name = unique_name("dynamic_base_struct");
    let enum_name = unique_name("dynamic_bounded_enum");
    let derived_name = unique_name("dynamic_derived_struct");
    let topic_name = unique_name("dynamic_derived_topic");

    let mut base_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(base_name.clone()))
        .unwrap();
    base_type
        .add_member(DynamicMemberBuilder::primitive("base_id", DynamicPrimitiveKind::UInt32).id(1))
        .unwrap();

    let mut enum_type = participant
        .create_dynamic_type(DynamicTypeBuilder::enumeration(enum_name.clone()))
        .unwrap();
    enum_type.set_bit_bound(8).unwrap();
    enum_type
        .add_enum_literal("Low", DynamicEnumLiteralValue::Explicit(1), false)
        .unwrap();
    enum_type
        .add_enum_literal("High", DynamicEnumLiteralValue::Explicit(200), true)
        .unwrap();

    let mut derived_type = participant
        .create_dynamic_type(
            DynamicTypeBuilder::structure(derived_name.clone()).base_type(base_type.as_spec()),
        )
        .unwrap();
    derived_type
        .set_extensibility(DynamicTypeExtensibility::Appendable)
        .unwrap();
    derived_type
        .add_member(DynamicMemberBuilder::new("level", enum_type.as_spec()).id(2))
        .unwrap();
    derived_type
        .add_member(DynamicMemberBuilder::primitive("value", DynamicPrimitiveKind::Float64).id(3))
        .unwrap();

    let mut duplicate = derived_type.duplicate().unwrap();
    let descriptor = duplicate
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(descriptor.type_name(), derived_name);
    assert!(descriptor.op_count() > 0);
    let topic = participant
        .create_topic_from_descriptor(&topic_name, &descriptor)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), derived_name);
}

#[test]
fn test_topic_descriptor_metadata_and_entity_sertype_access() {
    let participant = DomainParticipant::new(121).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_name("test_topic_descriptor_metadata_and_entity_sertype_access");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    assert!(!topic.get_sertype().unwrap().as_ptr().is_null());
    assert!(!writer.get_sertype().unwrap().as_ptr().is_null());
    assert!(!reader.get_sertype().unwrap().as_ptr().is_null());

    let mut dynamic_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(unique_name(
            "descriptor_meta_type",
        )))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(10))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("value", DynamicPrimitiveKind::Float64).id(20))
        .unwrap();
    dynamic_type.set_member_key(10, true).unwrap();

    let descriptor = dynamic_type
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(descriptor.key_count(), 1);
    assert!(!descriptor.ops().is_empty());
    assert!(!descriptor.key_descriptors().is_empty());
    assert_eq!(descriptor.key_descriptors()[0].name, "id");
    let _ = descriptor.flagset();
    let _ = descriptor.metadata_xml();
    let _ = descriptor.type_information_bytes();
    let _ = descriptor.type_mapping_bytes();
    let _ = descriptor.restrict_data_representation();
}

#[test]
fn test_type_info_can_create_topic_directly() {
    let participant = DomainParticipant::new(122).unwrap();
    let type_name = unique_name("type_info_create_topic_type");
    let topic_name = unique_name("type_info_create_topic_topic");

    let mut dynamic_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(type_name.clone()))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(1))
        .unwrap();
    dynamic_type.set_member_key(1, true).unwrap();

    let type_info = dynamic_type.register().unwrap();
    let topic = participant
        .create_topic_from_type_info(&topic_name, &type_info, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), type_name);
}

#[test]
fn test_dynamic_builder_applies_properties() {
    let participant = DomainParticipant::new(112).unwrap();
    let base_name = unique_name("builder_base_type");
    let enum_name = unique_name("builder_enum_type");
    let derived_name = unique_name("builder_derived_type");
    let topic_name = unique_name("builder_derived_topic");

    let base_type = participant
        .create_dynamic_type(
            DynamicTypeBuilder::structure(base_name.clone())
                .autoid(DynamicTypeAutoId::Hash)
                .extensibility(DynamicTypeExtensibility::Appendable),
        )
        .unwrap();

    let mut enum_type = participant
        .create_dynamic_type(DynamicTypeBuilder::enumeration(enum_name.clone()).bit_bound(8))
        .unwrap();
    enum_type
        .add_enum_literal("Off", DynamicEnumLiteralValue::Explicit(0), false)
        .unwrap();
    enum_type
        .add_enum_literal("On", DynamicEnumLiteralValue::Explicit(1), true)
        .unwrap();

    let mut derived_type = participant
        .create_dynamic_type(
            DynamicTypeBuilder::structure(derived_name.clone())
                .base_type(base_type.as_spec())
                .nested(true)
                .extensibility(DynamicTypeExtensibility::Appendable),
        )
        .unwrap();
    derived_type
        .add_member(DynamicMemberBuilder::new("state", enum_type.as_spec()).id(10))
        .unwrap();
    derived_type
        .add_member(DynamicMemberBuilder::primitive("value", DynamicPrimitiveKind::Float64).id(20))
        .unwrap();

    let descriptor = derived_type
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(descriptor.type_name(), derived_name);
    assert!(descriptor.op_count() > 0);
    let topic = participant
        .create_topic_from_descriptor(&topic_name, &descriptor)
        .unwrap();
    assert_eq!(topic.get_type_name().unwrap(), derived_name);
}

#[test]
fn test_dynamic_member_builder_applies_member_properties() {
    let participant = DomainParticipant::new(114).unwrap();
    let type_name = unique_name("dynamic_member_props_type");
    let topic_name = unique_name("dynamic_member_props_topic");

    let mut dynamic_type = participant
        .create_dynamic_type(
            DynamicTypeBuilder::structure(type_name.clone()).autoid(DynamicTypeAutoId::Hash),
        )
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(10)
                .key(),
        )
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("payload", DynamicPrimitiveKind::Float64)
                .id(20)
                .external()
                .must_understand(),
        )
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("renamed", DynamicPrimitiveKind::UInt16)
                .id(30)
                .hash_id("renamed_hash"),
        )
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("maybe", DynamicPrimitiveKind::UInt16)
                .id(40)
                .optional(),
        )
        .unwrap();

    let descriptor = dynamic_type
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(descriptor.type_name(), type_name);
    assert_eq!(descriptor.key_count(), 1);
    assert!(!descriptor.key_descriptors().is_empty());

    let topic = participant
        .create_topic_from_descriptor(&topic_name, &descriptor)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), type_name);
}

#[test]
fn test_dynamic_member_builder_requires_explicit_id_for_member_properties() {
    let participant = DomainParticipant::new(115).unwrap();
    let mut dynamic_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(unique_name(
            "dynamic_member_prop_error",
        )))
        .unwrap();
    let err = dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).key())
        .unwrap_err();
    assert!(matches!(err, DdsError::BadParameter(_)));
}

#[test]
fn test_topic_descriptor_can_create_topic_directly() {
    let participant = DomainParticipant::new(120).unwrap();
    let type_name = unique_name("descriptor_create_topic_type");
    let topic_name = unique_name("descriptor_create_topic_topic");

    let mut dynamic_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(type_name.clone()))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(1))
        .unwrap();
    dynamic_type.set_member_key(1, true).unwrap();

    let descriptor = dynamic_type
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    let topic = descriptor
        .create_topic(participant.entity(), &topic_name)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), type_name);
}

#[test]
fn test_dynamic_map_builder_reaches_runtime() {
    let participant = DomainParticipant::new(113).unwrap();
    let result = participant.create_dynamic_type(DynamicTypeBuilder::map(
        unique_name("dynamic_map_type"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::UInt32),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Float64),
        Some(16),
    ));
    match result {
        Ok(map_type) => {
            let _ = map_type.as_spec();
        }
        Err(DdsError::Unsupported(_)) | Err(DdsError::OutOfMemory) => {}
        Err(err) => panic!("unexpected map builder error: {err:?}"),
    }
}

#[test]
fn test_dynamic_union_member_properties_work() {
    let participant = DomainParticipant::new(119).unwrap();
    let union_name = unique_name("dynamic_union_member_props");
    let topic_name = unique_name("dynamic_union_member_props_topic");

    let mut union_type = participant
        .create_dynamic_type(
            DynamicTypeBuilder::union(
                union_name.clone(),
                DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
            )
            .autoid(DynamicTypeAutoId::Hash),
        )
        .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::Int32).labels(&[1, 2]),
        )
        .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("m2", DynamicPrimitiveKind::Int32)
                .labels(&[5])
                .id(777)
                .hash_id("m2_name")
                .external(),
        )
        .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("md", DynamicPrimitiveKind::Boolean).default_label(),
        )
        .unwrap();

    let descriptor = union_type
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(descriptor.type_name(), union_name);
    let topic = descriptor
        .create_topic(participant.entity(), &topic_name)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), union_name);
}

#[test]
fn test_dynamic_sequence_array_and_string_builders_work() {
    let participant = DomainParticipant::new(116).unwrap();
    let seq_name = unique_name("dynamic_seq_type");
    let arr_name = unique_name("dynamic_arr_type");
    let root_name = unique_name("dynamic_builder_root");
    let topic_name = unique_name("dynamic_builder_root_topic");

    let seq_type = participant
        .create_dynamic_type(DynamicTypeBuilder::sequence(
            seq_name.clone(),
            DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
            Some(8),
        ))
        .unwrap();
    let arr_type = participant
        .create_dynamic_type(DynamicTypeBuilder::array(
            arr_name.clone(),
            DynamicTypeSpec::primitive(DynamicPrimitiveKind::Float64),
            vec![2, 3],
        ))
        .unwrap();
    let string_type = participant
        .create_dynamic_type(DynamicTypeBuilder::string8(Some(24)))
        .unwrap();

    let mut root_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(root_name.clone()))
        .unwrap();
    root_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("values", seq_type.as_spec()).id(2))
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("matrix", arr_type.as_spec()).id(3))
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("label", string_type.as_spec()).id(4))
        .unwrap();

    let descriptor = root_type
        .register_topic_descriptor(&participant, FindScope::LocalDomain, 0)
        .unwrap();
    assert_eq!(descriptor.type_name(), root_name);
    assert_eq!(descriptor.key_count(), 1);
    let topic = descriptor
        .create_topic(participant.entity(), &topic_name)
        .unwrap();
    assert_eq!(topic.get_type_name().unwrap(), root_name);
}

#[test]
fn test_dynamic_member_insertion_positions_work() {
    let participant = DomainParticipant::new(123).unwrap();
    let type_name = unique_name("dynamic_member_positions");
    let topic_name = unique_name("dynamic_member_positions_topic");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name.clone())
        .autoid(DynamicTypeAutoId::Hash)
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive(
            "m1",
            DynamicPrimitiveKind::UInt16,
        ))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("m2", DynamicPrimitiveKind::UInt16).id(123))
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("m0", DynamicPrimitiveKind::UInt16)
                .at_start()
                .id(200),
        )
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("m3", DynamicPrimitiveKind::UInt16)
                .at_end()
                .id(201),
        )
        .unwrap();
    dynamic_type.set_member_key(200, true).unwrap();

    let topic = dynamic_type
        .register_topic(&participant, FindScope::LocalDomain, 0, &topic_name)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), type_name);
}

#[test]
fn test_dynamic_builder_build_and_register_topic_work() {
    let participant = DomainParticipant::new(124).unwrap();
    let type_name = unique_name("dynamic_builder_register_topic");
    let topic_name = unique_name("dynamic_builder_register_topic_topic");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name.clone())
        .extensibility(DynamicTypeExtensibility::Appendable)
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();

    let topic = dynamic_type
        .register_topic(&participant, FindScope::LocalDomain, 0, &topic_name)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), type_name);
}

#[test]
fn test_dynamic_invalid_hashid_conflict_is_reported() {
    let participant = DomainParticipant::new(125).unwrap();
    let mut dynamic_type = DynamicTypeBuilder::structure(unique_name("dynamic_hashid_conflict"))
        .autoid_hash()
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive(
            "m1",
            DynamicPrimitiveKind::UInt16,
        ))
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("m2", DynamicPrimitiveKind::UInt16)
                .id(500)
                .hash_id("m1"),
        )
        .unwrap_err();
}

#[test]
fn test_dynamic_invalid_union_duplicate_default_is_reported() {
    let participant = DomainParticipant::new(126).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_name("dynamic_union_default_conflict"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
    )
    .autoid_hash()
    .build(&participant)
    .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::Int32).union_default(),
        )
        .unwrap();
    let err = union_type
        .add_member(
            DynamicMemberBuilder::primitive("m2", DynamicPrimitiveKind::Boolean).union_default(),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_key_optional_combination_reaches_runtime() {
    let participant = DomainParticipant::new(127).unwrap();
    let type_name = unique_name("dynamic_key_optional_conflict");
    let topic_name = unique_name("dynamic_key_optional_conflict_topic");
    let mut dynamic_type = DynamicTypeBuilder::structure(type_name.clone())
        .autoid_hash()
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::UInt16)
                .id(10)
                .key()
                .optional(),
        )
        .unwrap();
    match dynamic_type.register_topic(&participant, FindScope::LocalDomain, 0, &topic_name) {
        Ok(topic) => {
            assert_eq!(topic.get_name().unwrap(), topic_name);
            assert_eq!(topic.get_type_name().unwrap(), type_name);
        }
        Err(DdsError::BadParameter(_)) | Err(DdsError::PreconditionNotMet(_)) => {}
        Err(err) => panic!("unexpected key+optional runtime error: {err:?}"),
    }
}

#[test]
fn test_dynamic_unbounded_builders_work() {
    let participant = DomainParticipant::new(128).unwrap();
    let seq_name = unique_name("dynamic_unbounded_seq_type");
    let root_name = unique_name("dynamic_unbounded_root_type");
    let topic_name = unique_name("dynamic_unbounded_root_topic");

    let seq_type = DynamicTypeBuilder::unbounded_sequence(
        seq_name.clone(),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
    )
    .build(&participant)
    .unwrap();
    let string_type = DynamicTypeBuilder::unbounded_string8()
        .build(&participant)
        .unwrap();

    let mut root_type = DynamicTypeBuilder::structure(root_name.clone())
        .build(&participant)
        .unwrap();
    root_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("values", seq_type.as_spec()).id(2))
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("label", string_type.as_spec()).id(3))
        .unwrap();

    let topic = root_type
        .register_topic(&participant, FindScope::LocalDomain, 0, &topic_name)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), root_name);
}

#[test]
fn test_dynamic_invalid_enum_bit_bound_is_reported() {
    let participant = DomainParticipant::new(129).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_name("dynamic_enum_bit_bound"))
        .bit_bound(2)
        .build(&participant)
        .unwrap();
    let err = enum_type
        .add_enum_literal("TooLarge", DynamicEnumLiteralValue::Explicit(4), false)
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_bitmask_auto_positions_work() {
    let participant = DomainParticipant::new(130).unwrap();
    let bitmask_name = unique_name("dynamic_bitmask_positions");
    let root_name = unique_name("dynamic_bitmask_root");
    let topic_name = unique_name("dynamic_bitmask_root_topic");

    let mut bitmask_type = DynamicTypeBuilder::bitmask(bitmask_name.clone())
        .bit_bound(16)
        .build(&participant)
        .unwrap();
    bitmask_type.add_bitmask_field("a", None).unwrap();
    bitmask_type.add_bitmask_field("b", Some(5)).unwrap();
    bitmask_type.add_bitmask_field("c", None).unwrap();

    let mut root_type = DynamicTypeBuilder::structure(root_name.clone())
        .build(&participant)
        .unwrap();
    root_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("flags", bitmask_type.as_spec()).id(2))
        .unwrap();

    let topic = root_type
        .register_topic(&participant, FindScope::LocalDomain, 0, &topic_name)
        .unwrap();
    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(topic.get_type_name().unwrap(), root_name);
}

#[test]
fn test_dynamic_struct_without_members_can_register() {
    let participant = DomainParticipant::new(131).unwrap();
    let type_name = unique_name("dynamic_empty_struct");
    let topic_name = unique_name("dynamic_empty_struct_topic");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name.clone())
        .build(&participant)
        .unwrap();
    let type_info = dynamic_type.register_type_info().unwrap();
    if let Ok(topic) =
        type_info.create_topic(participant.entity(), FindScope::LocalDomain, 0, &topic_name)
    {
        assert_eq!(topic.get_name().unwrap(), topic_name);
        assert_eq!(topic.get_type_name().unwrap(), type_name);
    }
}

#[test]
fn test_dynamic_union_without_members_reports_error() {
    let participant = DomainParticipant::new(132).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_name("dynamic_empty_union"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
    )
    .build(&participant)
    .unwrap();
    let result = union_type.register_type_info();
    assert!(matches!(
        result,
        Err(DdsError::BadParameter(_)) | Err(DdsError::PreconditionNotMet(_))
    ));
}

#[test]
fn test_dynamic_union_duplicate_label_is_reported() {
    let participant = DomainParticipant::new(133).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_name("dynamic_union_duplicate_label"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
    )
    .build(&participant)
    .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::Int32).union_labels(&[1]),
        )
        .unwrap();
    let err = union_type
        .add_member(
            DynamicMemberBuilder::primitive("m2", DynamicPrimitiveKind::Int32).union_labels(&[1]),
        )
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_register_type_info_alias_works() {
    let participant = DomainParticipant::new(134).unwrap();
    let type_name = unique_name("dynamic_register_type_info_alias");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name)
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(1))
        .unwrap();
    let type_info = dynamic_type.register_type_info().unwrap();
    let descriptor = type_info
        .create_topic_descriptor(participant.entity(), FindScope::LocalDomain, 0)
        .unwrap();
    assert!(descriptor.op_count() > 0);
}

#[test]
fn test_dynamic_invalid_struct_duplicate_member_id_is_reported() {
    let participant = DomainParticipant::new(135).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_name("dynamic_duplicate_member_id"))
            .build(&participant)
            .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::UInt16).id(1))
        .unwrap();
    let err = dynamic_type
        .add_member(DynamicMemberBuilder::primitive("m2", DynamicPrimitiveKind::UInt16).id(1))
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_union_hashid_conflict_is_reported() {
    let participant = DomainParticipant::new(136).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_name("dynamic_union_hashid_conflict"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
    )
    .autoid_hash()
    .build(&participant)
    .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::Int32).union_labels(&[1]),
        )
        .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("m2", DynamicPrimitiveKind::Int32)
                .id(700)
                .union_labels(&[2])
                .hash_id("m1"),
        )
        .unwrap_err();
}

#[test]
fn test_dynamic_struct_with_empty_substruct_reaches_runtime() {
    let participant = DomainParticipant::new(137).unwrap();
    let sub_name = unique_name("dynamic_empty_substruct");
    let root_name = unique_name("dynamic_struct_with_empty_substruct");
    let topic_name = unique_name("dynamic_struct_with_empty_substruct_topic");

    let sub_type = DynamicTypeBuilder::structure(sub_name)
        .build(&participant)
        .unwrap();
    let mut root_type = DynamicTypeBuilder::structure(root_name.clone())
        .build(&participant)
        .unwrap();
    root_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();
    root_type
        .add_member(DynamicMemberBuilder::new("child", sub_type.as_spec()).id(2))
        .unwrap();

    match root_type.register_topic(&participant, FindScope::LocalDomain, 0, &topic_name) {
        Ok(topic) => {
            assert_eq!(topic.get_name().unwrap(), topic_name);
            assert_eq!(topic.get_type_name().unwrap(), root_name);
        }
        Err(DdsError::BadParameter(_)) | Err(DdsError::PreconditionNotMet(_)) => {}
        Err(err) => panic!("unexpected empty-substruct runtime error: {err:?}"),
    }
}

#[test]
fn test_builtin_pseudo_topic_readers_work() {
    let participant = DomainParticipant::new(138).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_name("test_builtin_pseudo_topic_readers_work");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let _writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let _reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let participant_reader = participant.create_builtin_participant_reader().unwrap();
    let publication_reader = participant.create_builtin_publication_reader().unwrap();
    let subscription_reader = participant.create_builtin_subscription_reader().unwrap();

    thread::sleep(Duration::from_millis(500));

    let participants = participant_reader.read().unwrap();
    assert!(!participants.is_empty());
    assert!(participants
        .iter()
        .any(|sample| sample.qos().unwrap().is_some()));

    let publications = publication_reader.read().unwrap();
    assert!(publications
        .iter()
        .any(|sample| sample.topic_name() == topic_name));
    let pub_sample = publications
        .iter()
        .find(|sample| sample.topic_name() == topic_name)
        .unwrap();
    assert_eq!(pub_sample.type_name_value(), "DirectStringMessage");

    let subscriptions = subscription_reader.read().unwrap();
    assert!(subscriptions
        .iter()
        .any(|sample| sample.topic_name() == topic_name));
    let sub_sample = subscriptions
        .iter()
        .find(|sample| sample.topic_name() == topic_name)
        .unwrap();
    assert_eq!(sub_sample.type_name_value(), "DirectStringMessage");

    if let Ok(topic_reader) = participant.create_builtin_topic_reader() {
        let topics = topic_reader.read().unwrap();
        if let Some(sample) = topics
            .iter()
            .find(|sample| sample.topic_name() == topic_name)
        {
            assert_eq!(sample.type_name_value(), "DirectStringMessage");
        }
    }
}

#[test]
fn test_builtin_endpoint_sample_can_create_topic_descriptor_and_topic() {
    let participant1 = DomainParticipant::new(139).unwrap();
    let participant2 = DomainParticipant::new(139).unwrap();
    let publisher = participant1.create_publisher().unwrap();
    let subscriber = participant1.create_subscriber().unwrap();
    let topic_name =
        unique_name("test_builtin_endpoint_sample_can_create_topic_descriptor_and_topic");
    let topic = participant1
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let _writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let _reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let publication_reader = participant1.create_builtin_publication_reader().unwrap();
    let subscription_reader = participant1.create_builtin_subscription_reader().unwrap();
    thread::sleep(Duration::from_millis(500));

    let publications = publication_reader.read().unwrap();
    let publication = publications
        .iter()
        .find(|sample| sample.topic_name() == topic_name)
        .unwrap();
    assert_eq!(publication.type_name_value(), "DirectStringMessage");
    let _ = publication.qos().unwrap();
    if let Ok(descriptor) =
        publication.create_topic_descriptor(participant2.entity(), FindScope::LocalDomain, 0)
    {
        assert_eq!(descriptor.type_name(), "DirectStringMessage");
        let discovered = publication
            .create_topic(participant2.entity(), FindScope::LocalDomain, 0)
            .unwrap();
        assert_eq!(discovered.get_name().unwrap(), topic_name);
    }

    let subscriptions = subscription_reader.read().unwrap();
    let subscription = subscriptions
        .iter()
        .find(|sample| sample.topic_name() == topic_name)
        .unwrap();
    assert_eq!(subscription.type_name_value(), "DirectStringMessage");
    let _ = subscription.qos().unwrap();
    if let Ok(descriptor) =
        subscription.create_topic_descriptor(participant2.entity(), FindScope::LocalDomain, 0)
    {
        assert_eq!(descriptor.type_name(), "DirectStringMessage");
    }
}

#[test]
fn test_typeinfo_and_descriptor_can_create_topics_with_qos() {
    let participant = DomainParticipant::new(140).unwrap();
    let type_name = unique_name("typeinfo_qos_type");
    let topic_name_1 = unique_name("typeinfo_qos_topic_1");
    let topic_name_2 = unique_name("typeinfo_qos_topic_2");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name.clone())
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();
    let type_info = dynamic_type.register_type_info().unwrap();
    let descriptor = type_info
        .create_topic_descriptor(participant.entity(), FindScope::LocalDomain, 0)
        .unwrap();

    let qos1 = Qos::builder()
        .durability(Durability::TransientLocal)
        .build()
        .unwrap();
    let qos2 = Qos::builder()
        .durability(Durability::TransientLocal)
        .build()
        .unwrap();

    let topic1 = type_info
        .create_topic_with_qos(
            participant.entity(),
            FindScope::LocalDomain,
            0,
            &topic_name_1,
            &qos1,
        )
        .unwrap();
    let topic2 = descriptor
        .create_topic_with_qos(participant.entity(), &topic_name_2, &qos2)
        .unwrap();

    assert_eq!(topic1.get_name().unwrap(), topic_name_1);
    assert_eq!(topic2.get_name().unwrap(), topic_name_2);
    assert_eq!(
        topic1.get_qos().unwrap().durability().unwrap().unwrap(),
        Durability::TransientLocal
    );
    assert_eq!(
        topic2.get_qos().unwrap().durability().unwrap().unwrap(),
        Durability::TransientLocal
    );
}

#[test]
fn test_builtin_samples_can_find_existing_topics() {
    let participant1 = DomainParticipant::new(141).unwrap();
    let participant2 = DomainParticipant::new(141).unwrap();
    let publisher = participant1.create_publisher().unwrap();
    let subscriber = participant1.create_subscriber().unwrap();
    let topic_name = unique_name("test_builtin_samples_can_find_existing_topics");
    let topic = participant1
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let _writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let _reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let topic_reader = participant1.create_builtin_topic_reader();
    if let Ok(topic_reader) = topic_reader {
        let topics = topic_reader.read().unwrap();
        if let Some(topic_sample) = topics
            .iter()
            .find(|sample| sample.topic_name() == topic_name)
        {
            let found = topic_sample
                .find_topic(participant2.entity(), FindScope::LocalDomain, 0)
                .unwrap();
            if let Some(found) = found {
                assert_eq!(found.get_name().unwrap(), topic_name);
            }
        }
    }

    let publication_reader = participant1.create_builtin_publication_reader().unwrap();
    let publications = publication_reader.read().unwrap();
    let publication = publications
        .iter()
        .find(|sample| sample.topic_name() == topic_name)
        .unwrap();
    if let Ok(Some(found)) =
        publication.find_topic(participant2.entity(), FindScope::LocalDomain, 0)
    {
        assert_eq!(found.get_name().unwrap(), topic_name);
        assert_eq!(found.get_type_name().unwrap(), "DirectStringMessage");
    }
}

#[test]
fn test_sertype_hash_equality_and_topic_creation_work() {
    let participant = DomainParticipant::new(142).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_name("test_sertype_hash_equality_and_topic_creation_work");
    let cloned_topic_name = unique_name("test_sertype_clone_topic");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    let topic_st = topic.get_sertype().unwrap();
    let writer_st = writer.get_sertype().unwrap();
    let reader_st = reader.get_sertype().unwrap();

    assert!(topic_st.equals(&writer_st));
    assert!(topic_st.equals(&reader_st));
    assert_eq!(topic_st.hash(), writer_st.hash());
    assert_eq!(topic_st.hash(), reader_st.hash());

    let qos = Qos::builder()
        .durability(Durability::TransientLocal)
        .build()
        .unwrap();
    let cloned_topic = topic_st
        .clone_ref()
        .unwrap()
        .create_topic(participant.entity(), &cloned_topic_name, Some(&qos))
        .unwrap();
    assert_eq!(cloned_topic.get_name().unwrap(), cloned_topic_name);
    assert_eq!(cloned_topic.get_type_name().unwrap(), "DirectStringMessage");
    assert_eq!(
        cloned_topic
            .get_qos()
            .unwrap()
            .durability()
            .unwrap()
            .unwrap(),
        Durability::TransientLocal
    );
}

#[test]
fn test_typeinfo_typeids_are_accessible_and_comparable() {
    let participant = DomainParticipant::new(143).unwrap();
    let type_name = unique_name("typeinfo_typeids_dynamic");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name)
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();

    let info_a = dynamic_type.register_type_info().unwrap();
    let mut dup = dynamic_type.duplicate().unwrap();
    let info_b = dup.register_type_info().unwrap();

    assert!(info_a.is_present());
    assert!(info_a.is_valid());
    assert!(info_b.is_present());
    assert!(info_b.is_valid());

    let a_min = info_a.minimal_type_id().unwrap();
    let a_compl = info_a.complete_type_id().unwrap();
    let b_min = info_b.minimal_type_id().unwrap();
    let b_compl = info_b.complete_type_id().unwrap();

    assert!(a_min.is_minimal());
    assert!(a_compl.is_complete());
    assert_eq!(a_min.kind(), TypeIdKind::Minimal);
    assert_eq!(a_compl.kind(), TypeIdKind::Complete);
    assert!(a_min.equals(&b_min));
    assert!(a_compl.equals(&b_compl));
}

#[test]
fn test_participant_can_create_topic_from_sertype_with_qos() {
    let participant = DomainParticipant::new(144).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_name("test_participant_can_create_topic_from_sertype_with_qos");
    let clone_topic_name =
        unique_name("test_participant_can_create_topic_from_sertype_with_qos_clone");

    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    let topic_st = topic.get_sertype().unwrap();
    let writer_st = writer.get_sertype().unwrap();
    let reader_st = reader.get_sertype().unwrap();
    assert!(topic_st.equals(&writer_st));
    assert!(topic_st.equals(&reader_st));

    let qos = Qos::builder()
        .durability(Durability::TransientLocal)
        .build()
        .unwrap();
    let cloned_topic = participant
        .create_topic_from_sertype_with_qos(&clone_topic_name, &topic_st, &qos)
        .unwrap();
    assert_eq!(cloned_topic.get_name().unwrap(), clone_topic_name);
    assert_eq!(cloned_topic.get_type_name().unwrap(), "DirectStringMessage");
    assert_eq!(
        cloned_topic
            .get_qos()
            .unwrap()
            .durability()
            .unwrap()
            .unwrap(),
        Durability::TransientLocal
    );
}

#[test]
fn test_typeinfo_matches_and_resolves_type_objects() {
    let participant = DomainParticipant::new(145).unwrap();
    let type_name = unique_name("typeinfo_matches_type_objects");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name)
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();

    let info_a = dynamic_type.register_type_info().unwrap();
    let mut duplicate = dynamic_type.duplicate().unwrap();
    let info_b = duplicate.register_type_info().unwrap();

    assert!(info_a.matches(&info_b));
    let min_obj = info_a.minimal_type_object(participant.entity(), 0).unwrap();
    let compl_obj = info_a
        .complete_type_object(participant.entity(), 0)
        .unwrap();
    assert!(min_obj.is_some());
    assert!(compl_obj.is_some());

    let min_id = info_a.minimal_type_id().unwrap();
    let compl_id = info_a.complete_type_id().unwrap();
    let _ = min_id.resolve_type_object(participant.entity(), 0).unwrap();
    let _ = compl_id
        .resolve_type_object(participant.entity(), 0)
        .unwrap();
}

#[test]
fn test_typeinfo_and_matched_endpoint_can_find_topics_with_qos_variants() {
    let participant1 = DomainParticipant::new(146).unwrap();
    let participant2 = DomainParticipant::new(146).unwrap();
    let publisher = participant1.create_publisher().unwrap();
    let subscriber = participant1.create_subscriber().unwrap();
    let topic_name =
        unique_name("test_typeinfo_and_matched_endpoint_can_find_topics_with_qos_variants");
    let topic = participant1
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let _reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    if let Ok(type_info) = writer.get_type_info() {
        let found = type_info
            .find_topic(
                participant2.entity(),
                FindScope::LocalDomain,
                0,
                &topic_name,
            )
            .unwrap();
        if let Some(found) = found {
            assert_eq!(found.get_name().unwrap(), topic_name);
        }
    }

    let endpoint = writer
        .matched_subscription_endpoints()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    if let Ok(Some(found)) = endpoint.find_topic(participant2.entity(), FindScope::LocalDomain, 0) {
        assert_eq!(found.get_name().unwrap(), topic_name);
    }

    let qos = Qos::builder()
        .durability(Durability::TransientLocal)
        .build()
        .unwrap();
    if let Ok(cloned) =
        endpoint.create_topic_with_qos(participant2.entity(), FindScope::LocalDomain, 0, &qos)
    {
        assert_eq!(
            cloned.get_qos().unwrap().durability().unwrap().unwrap(),
            Durability::TransientLocal
        );
    }
}

#[test]
fn test_entity_and_endpoint_type_objects_match() {
    let participant = DomainParticipant::new(147).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_name("test_entity_and_endpoint_type_objects_match");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let _reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_name("entity_type_objects_dynamic"))
            .build(&participant)
            .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();
    let info_a = dynamic_type.register_type_info().unwrap();
    let mut dup = dynamic_type.duplicate().unwrap();
    let info_b = dup.register_type_info().unwrap();
    assert!(info_a.matches(&info_b));
    assert!(info_a
        .minimal_type_object(participant.entity(), 0)
        .unwrap()
        .is_some());
    assert!(info_a
        .complete_type_object(participant.entity(), 0)
        .unwrap()
        .is_some());

    let endpoint = writer
        .matched_subscription_endpoints()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    if let Ok(Some(_obj)) = endpoint.minimal_type_object(participant.entity(), 0) {}
    if let Ok(Some(_obj)) = endpoint.complete_type_object(participant.entity(), 0) {}
}

#[test]
fn test_builtin_endpoint_type_objects_are_accessible_when_available() {
    let participant = DomainParticipant::new(148).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name =
        unique_name("test_builtin_endpoint_type_objects_are_accessible_when_available");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let _writer =
        DataWriter::<DirectStringMessage>::new(publisher.entity(), topic.entity()).unwrap();
    let _reader =
        DataReader::<DirectStringMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    let publication_reader = participant.create_builtin_publication_reader().unwrap();
    thread::sleep(Duration::from_millis(500));
    let publications = publication_reader.read().unwrap();
    let publication = publications
        .iter()
        .find(|sample| sample.topic_name() == topic_name)
        .unwrap();

    if let Ok(info) = publication.type_info() {
        assert!(info.is_present());
        let _ = publication.minimal_type_object(participant.entity(), 0);
        let _ = publication.complete_type_object(participant.entity(), 0);
    }
}

#[test]
fn test_typeinfo_duplicate_equality_and_typeid_strings_work() {
    let participant = DomainParticipant::new(149).unwrap();
    let type_name = unique_name("typeinfo_duplicate_equality");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name)
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();

    let info = dynamic_type.register_type_info().unwrap();
    let dup = info.duplicate().unwrap();
    let _ = info.equals(&dup, TypeIncludeDeps::Ignore);
    let _ = info.equals(&dup, TypeIncludeDeps::Include);
    assert!(info.matches(&dup));

    let min = info.type_id(TypeIdKind::Minimal).unwrap();
    let compl = info.type_id(TypeIdKind::Complete).unwrap();
    let dup_min = dup.type_id(TypeIdKind::Minimal).unwrap();
    let dup_compl = dup.type_id(TypeIdKind::Complete).unwrap();
    assert!(min.equals(&dup_min));
    assert!(compl.equals(&dup_compl));
    assert!(!min.display_string().is_empty());
    assert!(!compl.display_string().is_empty());
    assert_eq!(min.equivalence_hash().len(), 14);
    assert_eq!(compl.equivalence_hash().len(), 14);
}

#[test]
fn test_dynamic_invalid_bitmask_duplicate_position_is_reported() {
    let participant = DomainParticipant::new(150).unwrap();
    let mut bitmask =
        DynamicTypeBuilder::bitmask(unique_name("dynamic_bitmask_duplicate_position"))
            .bit_bound(16)
            .build(&participant)
            .unwrap();
    bitmask.add_bitmask_field("b1", Some(1)).unwrap();
    let err = bitmask.add_bitmask_field("b2", Some(1)).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_empty_member_name_is_reported() {
    let participant = DomainParticipant::new(151).unwrap();
    let mut dynamic_type = DynamicTypeBuilder::structure(unique_name("dynamic_empty_member_name"))
        .build(&participant)
        .unwrap();
    let err = dynamic_type
        .add_member(DynamicMemberBuilder::primitive("", DynamicPrimitiveKind::UInt16).id(1))
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_set_extensibility_after_member_is_reported() {
    let participant = DomainParticipant::new(152).unwrap();
    let mut dynamic_type = DynamicTypeBuilder::structure(unique_name("dynamic_ext_after_member"))
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::UInt16).id(1))
        .unwrap();
    let err = dynamic_type
        .set_extensibility(DynamicTypeExtensibility::Appendable)
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::PreconditionNotMet(_) | DdsError::BadParameter(_)
    ));
}

#[test]
fn test_dynamic_invalid_set_autoid_after_member_is_reported() {
    let participant = DomainParticipant::new(153).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_name("dynamic_autoid_after_member"))
            .build(&participant)
            .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::UInt16).id(1))
        .unwrap();
    let err = dynamic_type
        .set_autoid(DynamicTypeAutoId::Hash)
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::PreconditionNotMet(_) | DdsError::BadParameter(_)
    ));
}

#[test]
fn test_dynamic_invalid_set_bit_bound_on_struct_is_reported() {
    let participant = DomainParticipant::new(154).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_name("dynamic_struct_bit_bound_invalid"))
            .build(&participant)
            .unwrap();
    let err = dynamic_type.set_bit_bound(16).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_owned_typeids_roundtrip_and_typeobject_resolution() {
    let participant = DomainParticipant::new(160).unwrap();
    let type_name = unique_name("owned_typeids_roundtrip");

    let mut dynamic_type = DynamicTypeBuilder::structure(type_name)
        .build(&participant)
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();

    let info = dynamic_type.register_type_info().unwrap();
    let min_ref = info.minimal_type_id().unwrap();
    let compl_ref = info.complete_type_id().unwrap();
    let min_owned = info.type_id_owned(TypeIdKind::Minimal).unwrap().unwrap();
    let compl_owned = info.type_id_owned(TypeIdKind::Complete).unwrap().unwrap();

    assert!(min_owned.equals_ref(&min_ref));
    assert!(compl_owned.equals_ref(&compl_ref));
    assert!(min_owned.is_minimal());
    assert!(compl_owned.is_complete());
    assert_eq!(min_owned.kind(), TypeIdKind::Minimal);
    assert_eq!(compl_owned.kind(), TypeIdKind::Complete);
    assert!(!min_owned.display_string().is_empty());
    assert!(!compl_owned.display_string().is_empty());
    let _ = min_owned.equivalence_hash();
    let _ = compl_owned.equivalence_hash();
    let _ = min_owned
        .resolve_type_object(participant.entity(), 0)
        .unwrap();
    let _ = compl_owned
        .resolve_type_object(participant.entity(), 0)
        .unwrap();
}

#[test]
fn test_dynamic_invalid_enum_empty_name_is_reported() {
    let participant = DomainParticipant::new(155).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_name("dynamic_enum_empty_name"))
        .build(&participant)
        .unwrap();
    let err = enum_type
        .add_enum_literal("", DynamicEnumLiteralValue::Explicit(1), false)
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_enum_duplicate_value_is_reported() {
    let participant = DomainParticipant::new(156).unwrap();
    let mut enum_type =
        DynamicTypeBuilder::enumeration(unique_name("dynamic_enum_duplicate_value"))
            .build(&participant)
            .unwrap();
    enum_type
        .add_enum_literal("e1", DynamicEnumLiteralValue::Explicit(1), false)
        .unwrap();
    let err = enum_type
        .add_enum_literal("e2", DynamicEnumLiteralValue::Explicit(1), false)
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_enum_duplicate_name_is_reported() {
    let participant = DomainParticipant::new(157).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_name("dynamic_enum_duplicate_name"))
        .build(&participant)
        .unwrap();
    enum_type
        .add_enum_literal("e1", DynamicEnumLiteralValue::Explicit(1), false)
        .unwrap();
    let err = enum_type
        .add_enum_literal("e1", DynamicEnumLiteralValue::Explicit(2), false)
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_enum_multiple_defaults_is_reported() {
    let participant = DomainParticipant::new(158).unwrap();
    let mut enum_type =
        DynamicTypeBuilder::enumeration(unique_name("dynamic_enum_multiple_defaults"))
            .build(&participant)
            .unwrap();
    enum_type
        .add_enum_literal("e1", DynamicEnumLiteralValue::Explicit(1), true)
        .unwrap();
    let err = enum_type
        .add_enum_literal("e2", DynamicEnumLiteralValue::Explicit(2), true)
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_bitmask_out_of_bound_position_is_reported() {
    let participant = DomainParticipant::new(159).unwrap();
    let mut bitmask = DynamicTypeBuilder::bitmask(unique_name("dynamic_bitmask_out_of_bound"))
        .bit_bound(2)
        .build(&participant)
        .unwrap();
    let err = bitmask.add_bitmask_field("b1", Some(2)).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_set_nested_on_enum_is_reported() {
    let participant = DomainParticipant::new(161).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_name("dynamic_set_nested_on_enum"))
        .build(&participant)
        .unwrap();
    let err = enum_type.set_nested(true).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_set_nested_after_member_is_reported() {
    let participant = DomainParticipant::new(162).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_name("dynamic_set_nested_after_member"))
            .build(&participant)
            .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::UInt16).id(1))
        .unwrap();
    let err = dynamic_type.set_nested(true).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_set_nested_after_register_is_reported() {
    let participant = DomainParticipant::new(163).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_name("dynamic_set_nested_after_register"))
            .build(&participant)
            .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("m1", DynamicPrimitiveKind::UInt16).id(1))
        .unwrap();
    let _ = dynamic_type.register_type_info().unwrap();
    let err = dynamic_type.set_nested(true).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_set_autoid_on_enum_is_reported() {
    let participant = DomainParticipant::new(164).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_name("dynamic_set_autoid_on_enum"))
        .build(&participant)
        .unwrap();
    let err = enum_type.set_autoid(DynamicTypeAutoId::Hash).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_dynamic_invalid_set_extensibility_on_primitive_is_reported() {
    let participant = DomainParticipant::new(165).unwrap();
    let mut string_type = DynamicTypeBuilder::unbounded_string8()
        .build(&participant)
        .unwrap();
    let err = string_type
        .set_extensibility(DynamicTypeExtensibility::Final)
        .unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn test_typeinfo_matches_false_for_different_types() {
    let participant = DomainParticipant::new(166).unwrap();
    let mut a = DynamicTypeBuilder::structure(unique_name("typeinfo_matches_false_a"))
        .build(&participant)
        .unwrap();
    a.add_member(
        DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
            .id(1)
            .key(),
    )
    .unwrap();
    let mut b = DynamicTypeBuilder::structure(unique_name("typeinfo_matches_false_b"))
        .build(&participant)
        .unwrap();
    b.add_member(DynamicMemberBuilder::primitive("value", DynamicPrimitiveKind::Float64).id(1))
        .unwrap();
    let info_a = a.register_type_info().unwrap();
    let info_b = b.register_type_info().unwrap();
    assert!(!info_a.matches(&info_b));
}

#[test]
fn test_qos_type_consistency_and_data_representation() {
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
fn test_entity_qos_roundtrip_for_type_consistency_and_data_representation() {
    let participant = DomainParticipant::new(167).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectStringMessage>(&unique_name(
            "test_entity_qos_roundtrip_for_type_consistency_and_data_representation",
        ))
        .unwrap();

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

    let writer =
        DataWriter::<DirectStringMessage>::with_qos(publisher.entity(), topic.entity(), Some(&qos))
            .unwrap();
    let reader = DataReader::<DirectStringMessage>::with_qos(
        subscriber.entity(),
        topic.entity(),
        Some(&qos),
    )
    .unwrap();

    if let Some(tc) = writer.get_qos().unwrap().type_consistency().unwrap() {
        assert_eq!(tc.kind, TypeConsistency::AllowTypeCoercion);
        assert!(tc.ignore_sequence_bounds);
        assert!(!tc.ignore_string_bounds);
        assert!(tc.ignore_member_names);
        assert!(!tc.prevent_type_widening);
        assert!(tc.force_type_validation);
    }
    if let Some(dr) = reader.get_qos().unwrap().data_representation().unwrap() {
        assert_eq!(
            dr,
            vec![DataRepresentation::Xcdr1, DataRepresentation::Xcdr2]
        );
    }
}

#[test]
fn test_builtin_reader_with_qos_roundtrip() {
    let participant = DomainParticipant::new(168).unwrap();
    let qos = Qos::builder()
        .entity_name("builtin-reader")
        .reliable()
        .build()
        .unwrap();
    let reader = participant
        .create_builtin_publication_reader_with_qos(&qos)
        .unwrap();
    let got = reader.get_qos().unwrap();
    assert_eq!(got.entity_name().unwrap().unwrap(), "builtin-reader");
    assert_eq!(got.reliability().unwrap().unwrap().0, Reliability::Reliable);
}

#[test]
fn test_qos_partition_resource_limits_time_filter_and_psmx() {
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
fn test_qos_property_and_binary_property_roundtrip() {
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
fn test_builtin_reader_qos_matches_discovered_endpoint_qos() {
    let participant = DomainParticipant::new(169).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_name("test_builtin_reader_qos_matches_discovered_endpoint_qos");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let reader_qos = Qos::builder()
        .entity_name("builtin-sub-reader")
        .build()
        .unwrap();
    let writer_qos = Qos::builder()
        .entity_name("builtin-pub-writer")
        .build()
        .unwrap();
    let writer = DataWriter::<DirectStringMessage>::with_qos(
        publisher.entity(),
        topic.entity(),
        Some(&writer_qos),
    )
    .unwrap();
    let _reader = DataReader::<DirectStringMessage>::with_qos(
        subscriber.entity(),
        topic.entity(),
        Some(&reader_qos),
    )
    .unwrap();

    thread::sleep(Duration::from_millis(500));

    let builtin_pub = participant.create_builtin_publication_reader().unwrap();
    let builtin_sub = participant.create_builtin_subscription_reader().unwrap();
    thread::sleep(Duration::from_millis(500));

    let publication = builtin_pub
        .read()
        .unwrap()
        .into_iter()
        .find(|sample| sample.topic_name() == topic_name)
        .unwrap();
    let subscription = builtin_sub
        .read()
        .unwrap()
        .into_iter()
        .find(|sample| sample.topic_name() == topic_name)
        .unwrap();

    assert_eq!(
        publication
            .qos()
            .unwrap()
            .unwrap()
            .entity_name()
            .unwrap()
            .unwrap(),
        "builtin-pub-writer"
    );
    assert_eq!(
        subscription
            .qos()
            .unwrap()
            .unwrap()
            .entity_name()
            .unwrap()
            .unwrap(),
        "builtin-sub-reader"
    );

    let matched_sub = writer
        .matched_subscription_endpoints()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(
        matched_sub
            .qos()
            .unwrap()
            .unwrap()
            .entity_name()
            .unwrap()
            .unwrap(),
        "builtin-sub-reader"
    );
}

#[test]
fn test_builtin_sample_qos_matches_entity_qos() {
    let participant = DomainParticipant::new(170).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_name("test_builtin_sample_qos_matches_entity_qos");

    let writer_qos = Qos::builder()
        .entity_name("builtin-writer-qos")
        .reliable()
        .build()
        .unwrap();
    let reader_qos = Qos::builder()
        .entity_name("builtin-reader-qos")
        .best_effort()
        .build()
        .unwrap();
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer = DataWriter::<DirectStringMessage>::with_qos(
        publisher.entity(),
        topic.entity(),
        Some(&writer_qos),
    )
    .unwrap();
    let reader = DataReader::<DirectStringMessage>::with_qos(
        subscriber.entity(),
        topic.entity(),
        Some(&reader_qos),
    )
    .unwrap();

    thread::sleep(Duration::from_millis(500));

    let builtin_pub = participant.create_builtin_publication_reader().unwrap();
    let builtin_sub = participant.create_builtin_subscription_reader().unwrap();
    let builtin_participant = participant.create_builtin_participant_reader().unwrap();
    thread::sleep(Duration::from_millis(500));

    let pub_sample = builtin_pub
        .read()
        .unwrap()
        .into_iter()
        .find(|s| s.topic_name() == topic_name)
        .unwrap();
    let sub_sample = builtin_sub
        .read()
        .unwrap()
        .into_iter()
        .find(|s| s.topic_name() == topic_name)
        .unwrap();
    assert!(pub_sample
        .qos()
        .unwrap()
        .unwrap()
        .equals(&writer.get_qos().unwrap()));
    assert!(sub_sample
        .qos()
        .unwrap()
        .unwrap()
        .equals(&reader.get_qos().unwrap()));

    let participant_guid = participant.get_guid().unwrap();
    let participant_sample = builtin_participant
        .read()
        .unwrap()
        .into_iter()
        .find(|s| s.key().v == participant_guid.v)
        .unwrap();
    let _ = participant_sample.qos().unwrap().unwrap();

    if let Ok(topic_reader) = participant.create_builtin_topic_reader() {
        let samples = topic_reader.read().unwrap();
        if let Some(topic_sample) = samples.into_iter().find(|s| s.topic_name() == topic_name) {
            assert!(topic_sample
                .qos()
                .unwrap()
                .unwrap()
                .equals(&topic.get_qos().unwrap()));
        }
    }
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

#[test]
fn test_dynamic_value_schema_validation_for_structs() {
    let participant = DomainParticipant::new(171).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_name("dynamic_value_struct_schema"))
            .build(&participant)
            .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::new(
                "label",
                DynamicTypeSpec::primitive(DynamicPrimitiveKind::UInt16),
            )
            .id(2)
            .optional(),
        )
        .unwrap();
    let seq_type = DynamicTypeBuilder::bounded_sequence(
        unique_name("dynamic_value_seq_schema"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
        4,
    )
    .build(&participant)
    .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::new("values", seq_type.as_spec()).id(3))
        .unwrap();

    let schema = dynamic_type.schema().clone();
    let mut fields = std::collections::BTreeMap::new();
    fields.insert("id".to_string(), DynamicValue::UInt32(7));
    fields.insert("label".to_string(), DynamicValue::Null);
    fields.insert(
        "values".to_string(),
        DynamicValue::Sequence(vec![DynamicValue::Int32(1), DynamicValue::Int32(2)]),
    );
    let value = DynamicValue::Struct(fields);
    value.validate_against(&schema).unwrap();

    let mut bad_fields = std::collections::BTreeMap::new();
    bad_fields.insert("id".to_string(), DynamicValue::UInt32(7));
    bad_fields.insert(
        "values".to_string(),
        DynamicValue::Sequence(vec![
            DynamicValue::Int32(1),
            DynamicValue::Int32(2),
            DynamicValue::Int32(3),
            DynamicValue::Int32(4),
            DynamicValue::Int32(5),
        ]),
    );
    let bad = DynamicValue::Struct(bad_fields);
    assert!(bad.validate_against(&schema).is_err());
}

#[test]
fn test_dynamic_value_schema_validation_for_unions() {
    let participant = DomainParticipant::new(172).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_name("dynamic_value_union_schema"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
    )
    .build(&participant)
    .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("i", DynamicPrimitiveKind::Int32)
                .id(1)
                .union_labels(&[1, 2]),
        )
        .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("flag", DynamicPrimitiveKind::Boolean)
                .id(2)
                .union_default(),
        )
        .unwrap();

    let schema = union_type.schema().clone();
    let good = DynamicValue::Union {
        discriminator: 1,
        field: "i".to_string(),
        value: Box::new(DynamicValue::Int32(42)),
    };
    good.validate_against(&schema).unwrap();

    let default_case = DynamicValue::Union {
        discriminator: 99,
        field: "flag".to_string(),
        value: Box::new(DynamicValue::Bool(true)),
    };
    default_case.validate_against(&schema).unwrap();

    let bad = DynamicValue::Union {
        discriminator: 3,
        field: "i".to_string(),
        value: Box::new(DynamicValue::Int32(42)),
    };
    assert!(bad.validate_against(&schema).is_err());
}

#[test]
fn test_dynamic_value_default_construction_and_field_updates() {
    let participant = DomainParticipant::new(173).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_name("dynamic_value_default_struct"))
            .build(&participant)
            .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
                .id(1)
                .key(),
        )
        .unwrap();
    dynamic_type
        .add_member(
            DynamicMemberBuilder::new(
                "label",
                DynamicTypeSpec::primitive(DynamicPrimitiveKind::UInt16),
            )
            .id(2)
            .optional(),
        )
        .unwrap();
    let seq_type = DynamicTypeBuilder::bounded_sequence(
        unique_name("dynamic_value_default_seq"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
        2,
    )
    .build(&participant)
    .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::new("values", seq_type.as_spec()).id(3))
        .unwrap();

    let schema = dynamic_type.schema().clone();
    let mut value = DynamicValue::new(&schema);
    assert_eq!(value.field("id"), Some(&DynamicValue::UInt32(0)));
    assert_eq!(value.field("label"), Some(&DynamicValue::Null));
    value
        .set_field(&schema, "id", DynamicValue::UInt32(99))
        .unwrap();
    value
        .set_field(&schema, "label", DynamicValue::UInt16(7))
        .unwrap();

    let seq_schema = match &schema {
        DynamicTypeSchema::Struct { fields, .. } => fields
            .iter()
            .find(|f| f.name == "values")
            .unwrap()
            .value
            .clone(),
        _ => panic!("expected struct schema"),
    };
    let values = value.as_struct_mut().unwrap().get_mut("values").unwrap();
    values.push(&seq_schema, DynamicValue::Int32(10)).unwrap();
    values.push(&seq_schema, DynamicValue::Int32(20)).unwrap();
    assert!(values.push(&seq_schema, DynamicValue::Int32(30)).is_err());
    value.validate_against(&schema).unwrap();
}

#[test]
fn test_dynamic_value_union_constructor_and_accessors() {
    let participant = DomainParticipant::new(174).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_name("dynamic_value_union_builder"),
        DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32),
    )
    .build(&participant)
    .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("i", DynamicPrimitiveKind::Int32)
                .id(1)
                .union_labels(&[1]),
        )
        .unwrap();
    union_type
        .add_member(
            DynamicMemberBuilder::primitive("flag", DynamicPrimitiveKind::Boolean)
                .id(2)
                .union_default(),
        )
        .unwrap();

    let schema = union_type.schema().clone();
    let union_value = DynamicValue::union(&schema, 1, "i", DynamicValue::Int32(42)).unwrap();
    assert_eq!(union_value.union_discriminator(), Some(1));
    assert_eq!(union_value.union_field_name(), Some("i"));
    assert_eq!(
        union_value.union_field_value(),
        Some(&DynamicValue::Int32(42))
    );

    let default_value = DynamicValue::new(&schema);
    assert_eq!(default_value.union_field_name(), Some("flag"));
}

// ── Async iterator tests ──

#[cfg(feature = "async")]
#[tokio::test]
async fn test_async_read_aiter() {
    use futures_util::StreamExt;

    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<HelloWorld>("test_async_read_aiter")
        .unwrap();

    let writer = DataWriter::<HelloWorld>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<HelloWorld>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let mut msg = HelloWorld {
        id: 99,
        message: [0u8; 256],
    };
    let text = b"async iterator test";
    msg.message[..text.len()].copy_from_slice(text);
    writer.write(&msg).unwrap();

    thread::sleep(Duration::from_millis(200));

    let stream = reader.read_aiter();
    futures_util::pin_mut!(stream);
    let batch = stream.next().await;
    assert!(
        batch.is_some(),
        "Expected at least one batch from read_aiter"
    );
    let samples = batch.unwrap().unwrap();
    assert!(!samples.is_empty(), "Expected at least one sample");
    assert_eq!(samples[0].id, 99);
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_async_take_aiter() {
    use futures_util::StreamExt;

    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let topic = participant
        .create_topic::<HelloWorld>("test_async_take_aiter")
        .unwrap();

    let writer = DataWriter::<HelloWorld>::new(publisher.entity(), topic.entity()).unwrap();
    let reader = DataReader::<HelloWorld>::new(subscriber.entity(), topic.entity()).unwrap();

    thread::sleep(Duration::from_millis(500));

    let mut msg = HelloWorld {
        id: 42,
        message: [0u8; 256],
    };
    let text = b"async take iterator test";
    msg.message[..text.len()].copy_from_slice(text);
    writer.write(&msg).unwrap();

    thread::sleep(Duration::from_millis(200));

    let stream = reader.take_aiter();
    futures_util::pin_mut!(stream);
    let batch = stream.next().await;
    assert!(
        batch.is_some(),
        "Expected at least one batch from take_aiter"
    );
    let samples = batch.unwrap().unwrap();
    assert!(!samples.is_empty(), "Expected at least one sample");
    assert_eq!(samples[0].id, 42);
}
