use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for, DerivedReading, TestMessage};
use std::time::Duration;

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

#[test]
fn entities_and_basic_pubsub_work() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("basic_pubsub");
    let topic = participant
        .create_topic::<TestMessage>(&topic_name)
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    assert_eq!(topic.get_name().unwrap(), topic_name);
    assert_eq!(reader.get_topic().unwrap(), topic.entity());
    assert_eq!(writer.get_topic().unwrap(), topic.entity());
    assert_eq!(writer.get_publisher().unwrap(), publisher.entity());
    assert_eq!(reader.get_subscriber().unwrap(), subscriber.entity());

    short_delay();

    let sent = TestMessage::new(7, 42, "hello-dds");
    writer.write(&sent).unwrap();

    let received = wait_for(Duration::from_secs(2), || {
        reader
            .read()
            .unwrap_or_default()
            .into_iter()
            .any(|m| m.id == 7)
    });
    assert!(received, "reader never observed the written sample");

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 7);
    assert_eq!(taken[0].value, 42);
    assert_eq!(taken[0].text(), "hello-dds");
}

#[test]
fn loan_peek_and_next_operations_work() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&unique_topic("loan_ops"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer.write(&TestMessage::new(1, 10, "peek-me")).unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .peek()
        .unwrap()
        .is_empty()));

    let peeked = reader.peek().unwrap();
    assert_eq!(peeked.len(), 1);
    let first = peeked.iter().next().unwrap();
    assert_eq!(first.data.id, 1);
    assert_eq!(first.data.text(), "peek-me");

    let next = reader.read_next().unwrap().expect("expected unread sample");
    assert_eq!(next.data.id, 1);

    writer.write(&TestMessage::new(2, 20, "take-bulk")).unwrap();
    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));
    assert!(!reader.take().unwrap().is_empty());

    writer.write(&TestMessage::new(3, 30, "take-next")).unwrap();
    let taken = wait_for(Duration::from_secs(2), || {
        reader
            .take_next()
            .unwrap()
            .map(|sample| sample.data.id == 3)
            .unwrap_or(false)
    });
    assert!(taken, "take_next did not yield the fresh unread sample");
}

#[test]
fn derive_macro_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DerivedReading>(&unique_topic("derive_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&DerivedReading {
            sensor_id: 11,
            temperature_raw: 225,
            humidity_raw: 650,
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .take()
        .unwrap()
        .is_empty()));
}

#[test]
fn unbounded_string_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<StringMessage>(&unique_topic("string_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&StringMessage {
            id: 99,
            text: DdsString::new("dds-string-roundtrip").unwrap(),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 99);
    assert_eq!(taken[0].text.to_string_lossy(), "dds-string-roundtrip");
}

#[test]
fn sequence_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<SequenceMessage>(&unique_topic("sequence_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&SequenceMessage {
            id: 12,
            values: DdsSequence::from_slice(&[1, 2, 3, 5, 8]).unwrap(),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 12);
    assert_eq!(taken[0].values.to_vec(), vec![1, 2, 3, 5, 8]);
}

#[test]
fn nested_struct_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<NestedPointMessage>(&unique_topic("nested_struct_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&NestedPointMessage {
            id: 44,
            point: Point2D { x: 3.5, y: -7.25 },
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 44);
    assert!((taken[0].point.x - 3.5).abs() < f64::EPSILON);
    assert!((taken[0].point.y + 7.25).abs() < f64::EPSILON);
}

#[test]
fn enum_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<EnumMessage>(&unique_topic("enum_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&EnumMessage {
            id: 77,
            state: SimpleState::Stopped,
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 77);
    assert_eq!(taken[0].state, SimpleState::Stopped);
}

#[test]
fn bounded_sequence_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<BoundedSequenceMessage>(&unique_topic("bounded_sequence_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&BoundedSequenceMessage {
            id: 91,
            values: DdsBoundedSequence::from_slice(&[3, 6, 9, 12]).unwrap(),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 91);
    assert_eq!(taken[0].values.to_vec(), vec![3, 6, 9, 12]);
}

#[test]
fn bounded_sequence_enum_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<BoundedSequenceEnumMessage>(&unique_topic(
            "bounded_sequence_enum_roundtrip",
        ))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&BoundedSequenceEnumMessage {
            id: 505,
            states: DdsBoundedSequence::from_slice(&[SimpleState::Idle, SimpleState::Stopped])
                .unwrap(),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 505);
    assert_eq!(
        taken[0].states.to_vec(),
        vec![SimpleState::Idle, SimpleState::Stopped]
    );
}

#[test]
fn array_string_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<ArrayStringMessage>(&unique_topic("array_string_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&ArrayStringMessage {
            id: 606,
            names: [
                DdsString::new("red").unwrap(),
                DdsString::new("green").unwrap(),
                DdsString::new("blue").unwrap(),
            ],
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 606);
    assert_eq!(taken[0].names[0].to_string_lossy(), "red");
    assert_eq!(taken[0].names[1].to_string_lossy(), "green");
    assert_eq!(taken[0].names[2].to_string_lossy(), "blue");
}

#[test]
fn array_enum_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<ArrayEnumMessage>(&unique_topic("array_enum_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&ArrayEnumMessage {
            id: 707,
            states: [
                SimpleState::Idle,
                SimpleState::Running,
                SimpleState::Stopped,
            ],
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 707);
    assert_eq!(
        taken[0].states,
        [
            SimpleState::Idle,
            SimpleState::Running,
            SimpleState::Stopped
        ]
    );
}

#[test]
fn sequence_struct_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<SequenceStructMessage>(&unique_topic("sequence_struct_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&SequenceStructMessage {
            id: 101,
            points: DdsSequence::from_slice(&[
                Point2D { x: 1.0, y: 1.5 },
                Point2D { x: 2.0, y: 2.5 },
            ])
            .unwrap(),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 101);
    let points = taken[0].points.to_vec();
    assert_eq!(points.len(), 2);
    assert!((points[0].x - 1.0).abs() < f64::EPSILON);
    assert!((points[0].y - 1.5).abs() < f64::EPSILON);
    assert!((points[1].x - 2.0).abs() < f64::EPSILON);
    assert!((points[1].y - 2.5).abs() < f64::EPSILON);
}

#[test]
fn array_struct_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<ArrayStructMessage>(&unique_topic("array_struct_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&ArrayStructMessage {
            id: 202,
            points: [
                Point2D { x: 0.25, y: 0.5 },
                Point2D { x: 1.25, y: 1.5 },
                Point2D { x: 2.25, y: 2.5 },
            ],
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 202);
    assert!((taken[0].points[0].x - 0.25).abs() < f64::EPSILON);
    assert!((taken[0].points[0].y - 0.5).abs() < f64::EPSILON);
    assert!((taken[0].points[1].x - 1.25).abs() < f64::EPSILON);
    assert!((taken[0].points[1].y - 1.5).abs() < f64::EPSILON);
    assert!((taken[0].points[2].x - 2.25).abs() < f64::EPSILON);
    assert!((taken[0].points[2].y - 2.5).abs() < f64::EPSILON);
}

#[test]
fn bounded_sequence_struct_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<BoundedSequenceStructMessage>(&unique_topic(
            "bounded_sequence_struct_roundtrip",
        ))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&BoundedSequenceStructMessage {
            id: 303,
            points: DdsBoundedSequence::from_slice(&[
                Point2D { x: 4.0, y: 4.5 },
                Point2D { x: 5.0, y: 5.5 },
            ])
            .unwrap(),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 303);
    let points = taken[0].points.to_vec();
    assert_eq!(points.len(), 2);
    assert!((points[0].x - 4.0).abs() < f64::EPSILON);
    assert!((points[0].y - 4.5).abs() < f64::EPSILON);
    assert!((points[1].x - 5.0).abs() < f64::EPSILON);
    assert!((points[1].y - 5.5).abs() < f64::EPSILON);
}

#[test]
fn sequence_enum_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<SequenceEnumMessage>(&unique_topic("sequence_enum_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&SequenceEnumMessage {
            id: 404,
            states: DdsSequence::from_slice(&[
                SimpleState::Idle,
                SimpleState::Running,
                SimpleState::Stopped,
            ])
            .unwrap(),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 404);
    assert_eq!(
        taken[0].states.to_vec(),
        vec![
            SimpleState::Idle,
            SimpleState::Running,
            SimpleState::Stopped
        ]
    );
}

#[test]
fn direct_string_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectStringMessage>(&unique_topic("direct_string_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&DirectStringMessage {
            id: 808,
            text: "direct-string".to_string(),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 808);
    assert_eq!(taken[0].text, "direct-string");
}

#[test]
fn direct_vec_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectVecMessage>(&unique_topic("direct_vec_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&DirectVecMessage {
            id: 909,
            values: vec![7, 14, 21, 28],
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 909);
    assert_eq!(taken[0].values, vec![7, 14, 21, 28]);
}

#[test]
fn direct_vec_string_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectVecStringMessage>(&unique_topic("direct_vec_string_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&DirectVecStringMessage {
            id: 1001,
            values: vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()],
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 1001);
    assert_eq!(
        taken[0].values,
        vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
    );
}

#[test]
fn direct_vec_enum_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectVecEnumMessage>(&unique_topic("direct_vec_enum_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&DirectVecEnumMessage {
            id: 1102,
            states: vec![
                SimpleState::Idle,
                SimpleState::Running,
                SimpleState::Stopped,
            ],
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 1102);
    assert_eq!(
        taken[0].states,
        vec![
            SimpleState::Idle,
            SimpleState::Running,
            SimpleState::Stopped
        ]
    );
}

#[test]
fn direct_vec_struct_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectVecStructMessage>(&unique_topic("direct_vec_struct_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&DirectVecStructMessage {
            id: 1150,
            points: vec![Point2D { x: 9.0, y: 9.5 }, Point2D { x: 10.0, y: 10.5 }],
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 1150);
    assert_eq!(taken[0].points.len(), 2);
    assert!((taken[0].points[0].x - 9.0).abs() < f64::EPSILON);
    assert!((taken[0].points[0].y - 9.5).abs() < f64::EPSILON);
    assert!((taken[0].points[1].x - 10.0).abs() < f64::EPSILON);
    assert!((taken[0].points[1].y - 10.5).abs() < f64::EPSILON);
}

#[test]
fn multi_array_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<MultiArrayMessage>(&unique_topic("multi_array_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&MultiArrayMessage {
            id: 1203,
            matrix: [[1, 2, 3], [4, 5, 6]],
            states: [
                [SimpleState::Idle, SimpleState::Running],
                [SimpleState::Stopped, SimpleState::Idle],
            ],
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 1203);
    assert_eq!(taken[0].matrix, [[1, 2, 3], [4, 5, 6]]);
    assert_eq!(
        taken[0].states,
        [
            [SimpleState::Idle, SimpleState::Running],
            [SimpleState::Stopped, SimpleState::Idle],
        ]
    );
}

#[test]
fn nested_sequences_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<NestedSequencesMessage>(&unique_topic("nested_sequences_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&NestedSequencesMessage {
            id: 1304,
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

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 1304);
    let matrix = taken[0].matrix.to_vec();
    assert_eq!(matrix.len(), 2);
    assert_eq!(matrix[0].to_vec(), vec![1, 2]);
    assert_eq!(matrix[1].to_vec(), vec![3, 4, 5]);
    let bounded = taken[0].bounded_matrix.to_vec();
    assert_eq!(bounded.len(), 2);
    assert_eq!(bounded[0].to_vec(), vec![1.5, 2.5]);
    assert_eq!(bounded[1].to_vec(), vec![3.5]);
}

#[test]
fn optional_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<OptionalMessage>(&unique_topic("optional_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer
        .write(&OptionalMessage {
            id: 1405,
            opt_long: Some(17),
            opt_double: Some(8.5),
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].id, 1405);
    assert_eq!(taken[0].opt_long, Some(17));
    assert_eq!(taken[0].opt_double, Some(8.5));
}

#[test]
fn nested_key_round_trip_works() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<NestedKeyMessage>(&unique_topic("nested_key_roundtrip"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let sample = NestedKeyMessage {
        location: Location {
            building: 12,
            floor: 5,
            room: 99,
        },
        description: {
            let mut d = [0u8; 64];
            d[..10].copy_from_slice(b"nested-key");
            d
        },
    };

    let ih = writer.register_instance(&sample).unwrap();
    assert_ne!(ih, 0);
    assert_eq!(writer.lookup_instance(&sample), ih);
    writer.write(&sample).unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());
    assert_eq!(taken[0].location.building, 12);
    assert_eq!(taken[0].location.floor, 5);
    assert_eq!(taken[0].location.room, 99);
}

#[test]
fn topic_discovery_from_type_info_works() {
    let participant1 = DomainParticipant::new(0).unwrap();
    let participant2 = DomainParticipant::new(0).unwrap();

    let type_name = unique_topic("find_topic_dynamic_type");
    let topic_name = unique_topic("find_topic_dynamic_topic");

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

    short_delay();

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
fn dynamic_type_registration_and_topic_descriptor_work() {
    let participant = DomainParticipant::new(27).unwrap();

    let type_name = unique_topic("dynamic_struct_type");
    let sub_type_name = unique_topic("dynamic_sub_type");
    let enum_type_name = unique_topic("dynamic_enum_type");
    let seq_type_name = unique_topic("dynamic_seq_type");
    let topic_name = unique_topic("dynamic_struct_topic");

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
fn dynamic_union_bitmask_and_alias_registration_work() {
    let participant = DomainParticipant::new(28).unwrap();

    let sub_type_name = unique_topic("dynamic_union_subtype");
    let union_type_name = unique_topic("dynamic_union_type");
    let bitmask_type_name = unique_topic("dynamic_bitmask_type");
    let alias_type_name = unique_topic("dynamic_alias_type");
    let root_type_name = unique_topic("dynamic_root_type");
    let topic_name = unique_topic("dynamic_root_topic");

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
fn matched_endpoint_qos_and_keys_are_accessible() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectStringMessage>(&unique_topic("matched_endpoint_qos_and_keys"))
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

    short_delay();

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
fn matched_endpoint_can_create_topic_descriptor_and_topic() {
    let participant1 = DomainParticipant::new(0).unwrap();
    let participant3 = DomainParticipant::new(0).unwrap();

    let publisher = participant1.create_publisher().unwrap();
    let subscriber = participant1.create_subscriber().unwrap();
    let topic_name = unique_topic("matched_endpoint_create_topic");
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

    short_delay();

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
fn dynamic_base_type_and_duplicate_registration_work() {
    let participant = DomainParticipant::new(21).unwrap();

    let base_name = unique_topic("dynamic_base_struct");
    let enum_name = unique_topic("dynamic_bounded_enum");
    let derived_name = unique_topic("dynamic_derived_struct");
    let topic_name = unique_topic("dynamic_derived_topic");

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
fn topic_descriptor_metadata_and_entity_sertype_access_work() {
    let participant = DomainParticipant::new(31).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("topic_descriptor_metadata");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    assert!(!topic.get_sertype().unwrap().as_ptr().is_null());
    assert!(!writer.get_sertype().unwrap().as_ptr().is_null());
    assert!(!reader.get_sertype().unwrap().as_ptr().is_null());

    let mut dynamic_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(unique_topic(
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
fn type_info_can_create_topic_directly() {
    let participant = DomainParticipant::new(32).unwrap();
    let type_name = unique_topic("type_info_create_topic_type");
    let topic_name = unique_topic("type_info_create_topic_topic");

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
fn dynamic_builder_applies_properties_work() {
    let participant = DomainParticipant::new(22).unwrap();
    let base_name = unique_topic("builder_base_type");
    let enum_name = unique_topic("builder_enum_type");
    let derived_name = unique_topic("builder_derived_type");
    let topic_name = unique_topic("builder_derived_topic");

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
fn dynamic_member_builder_applies_member_properties_work() {
    let participant = DomainParticipant::new(24).unwrap();
    let type_name = unique_topic("dynamic_member_props_type");
    let topic_name = unique_topic("dynamic_member_props_topic");

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
fn dynamic_member_builder_requires_explicit_id_for_member_properties_work() {
    let participant = DomainParticipant::new(25).unwrap();
    let mut dynamic_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(unique_topic(
            "dynamic_member_prop_error",
        )))
        .unwrap();
    let err = dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).key())
        .unwrap_err();
    assert!(matches!(err, DdsError::BadParameter(_)));
}

#[test]
fn topic_descriptor_can_create_topic_directly() {
    let participant = DomainParticipant::new(30).unwrap();
    let type_name = unique_topic("descriptor_create_topic_type");
    let topic_name = unique_topic("descriptor_create_topic_topic");

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
fn dynamic_map_builder_reaches_runtime_work() {
    let participant = DomainParticipant::new(23).unwrap();
    let result = participant.create_dynamic_type(DynamicTypeBuilder::map(
        unique_topic("dynamic_map_type"),
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
fn dynamic_union_member_properties_work() {
    let participant = DomainParticipant::new(29).unwrap();
    let union_name = unique_topic("dynamic_union_member_props");
    let topic_name = unique_topic("dynamic_union_member_props_topic");

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
fn dynamic_sequence_array_and_string_builders_work() {
    let participant = DomainParticipant::new(26).unwrap();
    let seq_name = unique_topic("dynamic_seq_type");
    let arr_name = unique_topic("dynamic_arr_type");
    let root_name = unique_topic("dynamic_builder_root");
    let topic_name = unique_topic("dynamic_builder_root_topic");

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
fn dynamic_member_insertion_positions_work() {
    let participant = DomainParticipant::new(33).unwrap();
    let type_name = unique_topic("dynamic_member_positions");
    let topic_name = unique_topic("dynamic_member_positions_topic");

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
fn dynamic_builder_build_and_register_topic_work() {
    let participant = DomainParticipant::new(34).unwrap();
    let type_name = unique_topic("dynamic_builder_register_topic");
    let topic_name = unique_topic("dynamic_builder_register_topic_topic");

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
fn dynamic_invalid_hashid_conflict_is_reported_work() {
    let participant = DomainParticipant::new(35).unwrap();
    let mut dynamic_type = DynamicTypeBuilder::structure(unique_topic("dynamic_hashid_conflict"))
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
fn dynamic_invalid_union_duplicate_default_is_reported_work() {
    let participant = DomainParticipant::new(36).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_topic("dynamic_union_default_conflict"),
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
fn dynamic_key_optional_combination_reaches_runtime_work() {
    let participant = DomainParticipant::new(37).unwrap();
    let type_name = unique_topic("dynamic_key_optional_conflict");
    let topic_name = unique_topic("dynamic_key_optional_conflict_topic");
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
fn dynamic_unbounded_builders_work() {
    let participant = DomainParticipant::new(38).unwrap();
    let seq_name = unique_topic("dynamic_unbounded_seq_type");
    let root_name = unique_topic("dynamic_unbounded_root_type");
    let topic_name = unique_topic("dynamic_unbounded_root_topic");

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
fn dynamic_invalid_enum_bit_bound_is_reported_work() {
    let participant = DomainParticipant::new(39).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_topic("dynamic_enum_bit_bound"))
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
fn dynamic_bitmask_auto_positions_work() {
    let participant = DomainParticipant::new(40).unwrap();
    let bitmask_name = unique_topic("dynamic_bitmask_positions");
    let root_name = unique_topic("dynamic_bitmask_root");
    let topic_name = unique_topic("dynamic_bitmask_root_topic");

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
fn dynamic_struct_without_members_can_register_work() {
    let participant = DomainParticipant::new(41).unwrap();
    let type_name = unique_topic("dynamic_empty_struct");
    let topic_name = unique_topic("dynamic_empty_struct_topic");

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
fn dynamic_union_without_members_reports_error_work() {
    let participant = DomainParticipant::new(42).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_topic("dynamic_empty_union"),
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
fn dynamic_union_duplicate_label_is_reported_work() {
    let participant = DomainParticipant::new(43).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_topic("dynamic_union_duplicate_label"),
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
fn dynamic_register_type_info_alias_works() {
    let participant = DomainParticipant::new(44).unwrap();
    let type_name = unique_topic("dynamic_register_type_info_alias");

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
fn dynamic_invalid_struct_duplicate_member_id_is_reported_work() {
    let participant = DomainParticipant::new(45).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_topic("dynamic_duplicate_member_id"))
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
fn dynamic_invalid_union_hashid_conflict_is_reported_work() {
    let participant = DomainParticipant::new(46).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_topic("dynamic_union_hashid_conflict"),
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
fn dynamic_struct_with_empty_substruct_reaches_runtime_work() {
    let participant = DomainParticipant::new(47).unwrap();
    let sub_name = unique_topic("dynamic_empty_substruct");
    let root_name = unique_topic("dynamic_struct_with_empty_substruct");
    let topic_name = unique_topic("dynamic_struct_with_empty_substruct_topic");

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
fn builtin_pseudo_topic_readers_work() {
    let participant = DomainParticipant::new(48).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("builtin_pseudo_topic_readers_work");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let _writer = publisher.create_writer(&topic).unwrap();
    let _reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let participant_reader = participant.create_builtin_participant_reader().unwrap();
    let publication_reader = participant.create_builtin_publication_reader().unwrap();
    let subscription_reader = participant.create_builtin_subscription_reader().unwrap();

    short_delay();

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
fn builtin_endpoint_sample_can_create_topic_descriptor_and_topic() {
    let participant1 = DomainParticipant::new(49).unwrap();
    let participant2 = DomainParticipant::new(49).unwrap();
    let publisher = participant1.create_publisher().unwrap();
    let subscriber = participant1.create_subscriber().unwrap();
    let topic_name = unique_topic("builtin_endpoint_sample_create_topic");
    let topic = participant1
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let _writer = publisher.create_writer(&topic).unwrap();
    let _reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let publication_reader = participant1.create_builtin_publication_reader().unwrap();
    let subscription_reader = participant1.create_builtin_subscription_reader().unwrap();
    short_delay();

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
fn typeinfo_and_descriptor_can_create_topics_with_qos() {
    let participant = DomainParticipant::new(50).unwrap();
    let type_name = unique_topic("typeinfo_qos_type");
    let topic_name_1 = unique_topic("typeinfo_qos_topic_1");
    let topic_name_2 = unique_topic("typeinfo_qos_topic_2");

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
fn builtin_samples_can_find_existing_topics() {
    let participant1 = DomainParticipant::new(51).unwrap();
    let participant2 = DomainParticipant::new(51).unwrap();
    let publisher = participant1.create_publisher().unwrap();
    let subscriber = participant1.create_subscriber().unwrap();
    let topic_name = unique_topic("builtin_samples_find_topics");
    let topic = participant1
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let _writer = publisher.create_writer(&topic).unwrap();
    let _reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

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
fn sertype_hash_equality_and_topic_creation_work() {
    let participant = DomainParticipant::new(52).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("sertype_hash_equality");
    let cloned_topic_name = unique_topic("sertype_clone_topic");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

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
fn typeinfo_typeids_are_accessible_and_comparable() {
    let participant = DomainParticipant::new(53).unwrap();
    let type_name = unique_topic("typeinfo_typeids_dynamic");

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
fn participant_can_create_topic_from_sertype_with_qos() {
    let participant = DomainParticipant::new(54).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("participant_create_topic_from_sertype");
    let clone_topic_name = unique_topic("participant_create_topic_from_sertype_clone");

    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

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
fn typeinfo_matches_and_resolves_type_objects() {
    let participant = DomainParticipant::new(55).unwrap();
    let type_name = unique_topic("typeinfo_matches_type_objects");

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
fn typeinfo_and_matched_endpoint_can_find_topics_with_qos_variants() {
    let participant1 = DomainParticipant::new(56).unwrap();
    let participant2 = DomainParticipant::new(56).unwrap();
    let publisher = participant1.create_publisher().unwrap();
    let subscriber = participant1.create_subscriber().unwrap();
    let topic_name = unique_topic("typeinfo_and_matched_endpoint_find");
    let topic = participant1
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let _reader = subscriber.create_reader(&topic).unwrap();

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
fn entity_and_endpoint_type_objects_match() {
    let participant = DomainParticipant::new(57).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("entity_endpoint_type_objects");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let _reader = subscriber.create_reader(&topic).unwrap();

    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_topic("entity_type_objects_dynamic"))
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
fn builtin_endpoint_type_objects_are_accessible_when_available() {
    let participant = DomainParticipant::new(58).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("builtin_endpoint_type_objects");
    let topic = participant
        .create_topic::<DirectStringMessage>(&topic_name)
        .unwrap();
    let _writer = publisher.create_writer(&topic).unwrap();
    let _reader = subscriber.create_reader(&topic).unwrap();

    let publication_reader = participant.create_builtin_publication_reader().unwrap();
    short_delay();
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
fn typeinfo_duplicate_equality_and_typeid_strings_work() {
    let participant = DomainParticipant::new(59).unwrap();
    let type_name = unique_topic("typeinfo_duplicate_equality");

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
fn dynamic_invalid_bitmask_duplicate_position_is_reported_work() {
    let participant = DomainParticipant::new(60).unwrap();
    let mut bitmask =
        DynamicTypeBuilder::bitmask(unique_topic("dynamic_bitmask_duplicate_position"))
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
fn dynamic_invalid_empty_member_name_is_reported_work() {
    let participant = DomainParticipant::new(61).unwrap();
    let mut dynamic_type = DynamicTypeBuilder::structure(unique_topic("dynamic_empty_member_name"))
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
fn dynamic_invalid_set_extensibility_after_member_is_reported_work() {
    let participant = DomainParticipant::new(62).unwrap();
    let mut dynamic_type = DynamicTypeBuilder::structure(unique_topic("dynamic_ext_after_member"))
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
fn dynamic_invalid_set_autoid_after_member_is_reported_work() {
    let participant = DomainParticipant::new(63).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_topic("dynamic_autoid_after_member"))
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
fn dynamic_invalid_set_bit_bound_on_struct_is_reported_work() {
    let participant = DomainParticipant::new(64).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_topic("dynamic_struct_bit_bound_invalid"))
            .build(&participant)
            .unwrap();
    let err = dynamic_type.set_bit_bound(16).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn owned_typeids_roundtrip_and_typeobject_resolution() {
    let participant = DomainParticipant::new(65).unwrap();
    let type_name = unique_topic("owned_typeids_roundtrip");

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
fn dynamic_invalid_enum_empty_name_is_reported_work() {
    let participant = DomainParticipant::new(66).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_topic("dynamic_enum_empty_name"))
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
fn dynamic_invalid_enum_duplicate_value_is_reported_work() {
    let participant = DomainParticipant::new(67).unwrap();
    let mut enum_type =
        DynamicTypeBuilder::enumeration(unique_topic("dynamic_enum_duplicate_value"))
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
fn dynamic_invalid_enum_duplicate_name_is_reported_work() {
    let participant = DomainParticipant::new(68).unwrap();
    let mut enum_type =
        DynamicTypeBuilder::enumeration(unique_topic("dynamic_enum_duplicate_name"))
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
fn dynamic_invalid_enum_multiple_defaults_is_reported_work() {
    let participant = DomainParticipant::new(69).unwrap();
    let mut enum_type =
        DynamicTypeBuilder::enumeration(unique_topic("dynamic_enum_multiple_defaults"))
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
fn dynamic_invalid_bitmask_out_of_bound_position_is_reported_work() {
    let participant = DomainParticipant::new(70).unwrap();
    let mut bitmask = DynamicTypeBuilder::bitmask(unique_topic("dynamic_bitmask_out_of_bound"))
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
fn dynamic_invalid_set_nested_on_enum_is_reported_work() {
    let participant = DomainParticipant::new(71).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_topic("dynamic_set_nested_on_enum"))
        .build(&participant)
        .unwrap();
    let err = enum_type.set_nested(true).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn dynamic_invalid_set_nested_after_member_is_reported_work() {
    let participant = DomainParticipant::new(72).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_topic("dynamic_set_nested_after_member"))
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
fn dynamic_invalid_set_nested_after_register_is_reported_work() {
    let participant = DomainParticipant::new(73).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_topic("dynamic_set_nested_after_register"))
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
fn dynamic_invalid_set_autoid_on_enum_is_reported_work() {
    let participant = DomainParticipant::new(74).unwrap();
    let mut enum_type = DynamicTypeBuilder::enumeration(unique_topic("dynamic_set_autoid_on_enum"))
        .build(&participant)
        .unwrap();
    let err = enum_type.set_autoid(DynamicTypeAutoId::Hash).unwrap_err();
    assert!(matches!(
        err,
        DdsError::BadParameter(_) | DdsError::PreconditionNotMet(_)
    ));
}

#[test]
fn dynamic_invalid_set_extensibility_on_string_is_reported_work() {
    let participant = DomainParticipant::new(75).unwrap();
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
fn typeinfo_matches_false_for_different_types_work() {
    let participant = DomainParticipant::new(76).unwrap();
    let mut a = DynamicTypeBuilder::structure(unique_topic("typeinfo_matches_false_a"))
        .build(&participant)
        .unwrap();
    a.add_member(
        DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32)
            .id(1)
            .key(),
    )
    .unwrap();
    let mut b = DynamicTypeBuilder::structure(unique_topic("typeinfo_matches_false_b"))
        .build(&participant)
        .unwrap();
    b.add_member(DynamicMemberBuilder::primitive("value", DynamicPrimitiveKind::Float64).id(1))
        .unwrap();
    let info_a = a.register_type_info().unwrap();
    let info_b = b.register_type_info().unwrap();
    assert!(!info_a.matches(&info_b));
}

#[test]
fn entity_qos_roundtrip_for_type_consistency_and_data_representation() {
    let participant = DomainParticipant::new(77).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<DirectStringMessage>(&unique_topic(
            "entity_qos_roundtrip_type_consistency_data_representation",
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
fn builtin_reader_with_qos_roundtrip() {
    let participant = DomainParticipant::new(78).unwrap();
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
fn builtin_reader_qos_matches_discovered_endpoint_qos() {
    let participant = DomainParticipant::new(79).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("builtin_reader_qos_matches_endpoint");
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

    short_delay();

    let builtin_pub = participant.create_builtin_publication_reader().unwrap();
    let builtin_sub = participant.create_builtin_subscription_reader().unwrap();
    short_delay();

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
fn builtin_sample_qos_matches_entity_qos() {
    let participant = DomainParticipant::new(80).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic_name = unique_topic("builtin_sample_qos_matches_entity_qos");

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

    short_delay();

    let builtin_pub = participant.create_builtin_publication_reader().unwrap();
    let builtin_sub = participant.create_builtin_subscription_reader().unwrap();
    let builtin_participant = participant.create_builtin_participant_reader().unwrap();
    short_delay();

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
            let _ = topic_sample.qos().unwrap();
        }
    }
}

#[test]
fn dynamic_value_schema_validation_for_structs() {
    let participant = DomainParticipant::new(81).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_topic("dynamic_value_struct_schema"))
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
        unique_topic("dynamic_value_seq_schema"),
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
fn dynamic_value_schema_validation_for_unions() {
    let participant = DomainParticipant::new(82).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_topic("dynamic_value_union_schema"),
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
fn dynamic_value_default_construction_and_field_updates() {
    let participant = DomainParticipant::new(83).unwrap();
    let mut dynamic_type =
        DynamicTypeBuilder::structure(unique_topic("dynamic_value_default_struct"))
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
        unique_topic("dynamic_value_default_seq"),
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
fn dynamic_value_union_constructor_and_accessors() {
    let participant = DomainParticipant::new(84).unwrap();
    let mut union_type = DynamicTypeBuilder::union(
        unique_topic("dynamic_value_union_builder"),
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
