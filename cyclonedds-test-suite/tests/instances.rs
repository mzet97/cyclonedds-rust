use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for, KeyedMessage};
use std::time::Duration;

#[test]
fn keyed_instances_support_register_lookup_key_and_lifecycle() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<KeyedMessage>(&unique_topic("instances"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let first = KeyedMessage {
        key: 10,
        value: 100,
    };
    let second = KeyedMessage {
        key: 20,
        value: 200,
    };

    let first_handle = writer.register_instance(&first).unwrap();
    let second_handle = writer.register_instance(&second).unwrap();
    assert_ne!(first_handle, 0);
    assert_ne!(second_handle, 0);
    assert_ne!(first_handle, second_handle);
    assert_eq!(writer.lookup_instance(&first), first_handle);
    assert_eq!(reader.lookup_instance(&first), first_handle);

    writer
        .write(&KeyedMessage {
            key: 10,
            value: 101,
        })
        .unwrap();
    writer
        .write(&KeyedMessage {
            key: 20,
            value: 201,
        })
        .unwrap();

    assert!(wait_for(Duration::from_secs(2), || reader
        .read()
        .unwrap()
        .len()
        >= 2));
    assert!(reader.take().unwrap().len() >= 2);

    let recovered = reader.instance_get_key(first_handle).unwrap();
    assert_eq!(recovered.key, 10);

    writer
        .write_dispose(&KeyedMessage {
            key: 10,
            value: 999,
        })
        .unwrap();
    writer.unregister_instance_handle(second_handle).unwrap();
}
