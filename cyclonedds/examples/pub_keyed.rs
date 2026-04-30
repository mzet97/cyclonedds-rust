use cyclonedds::*;

#[repr(C)]
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let publisher = Publisher::new(participant.entity())?;
    let topic = Topic::<KeyValue>::new(participant.entity(), "KeyValueTopic")?;
    let writer = DataWriter::new(publisher.entity(), topic.entity())?;

    let msg1 = KeyValue { key: 1, value: 100 };
    let msg2 = KeyValue { key: 2, value: 200 };

    let ih1 = writer.register_instance(&msg1)?;
    println!("[PUB] Registered key=1, handle={}", ih1);

    let ih2 = writer.register_instance(&msg2)?;
    println!("[PUB] Registered key=2, handle={}", ih2);

    for i in 0..5 {
        writer.write(&KeyValue {
            key: 1,
            value: 100 + i,
        })?;
        writer.write(&KeyValue {
            key: 2,
            value: 200 + i,
        })?;
        println!("[PUB] Wrote iteration {}", i);
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    writer.write_dispose(&KeyValue { key: 1, value: 999 })?;
    println!("[PUB] Writedispose key=1");

    writer.unregister_instance_handle(ih2)?;
    println!("[PUB] Unregistered key=2");

    std::thread::sleep(std::time::Duration::from_secs(2));
    Ok(())
}
