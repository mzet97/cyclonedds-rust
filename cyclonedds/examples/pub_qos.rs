use cyclonedds::*;

#[repr(C)]
struct HelloWorld {
    id: i32,
    message: [u8; 256],
}

impl DdsType for HelloWorld {
    fn type_name() -> &'static str {
        "HelloWorldQos"
    }
    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 0));
        ops.extend(adr_bst(4, 256));
        ops
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;

    // Writer com QoS: Reliable + TransientLocal + KeepLast(10)
    // Dados ficam retidos para readers que entram depois
    let qos = QosBuilder::new()
        .reliable()
        .transient_local()
        .keep_last(10)
        .build()?;

    let publisher = Publisher::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "HelloWorldQosTopic")?;
    let writer = publisher.create_writer_with_qos(&topic, &qos)?;

    let mut msg = HelloWorld {
        id: 0,
        message: [0u8; 256],
    };
    let text = b"Hello with QoS from Rust!";
    msg.message[..text.len()].copy_from_slice(text);

    for i in 0..10 {
        msg.id = i;
        writer.write(&msg)?;
        println!("[PUB QoS] Published id={}", i);
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    println!("[PUB QoS] Done. Data retained with TransientLocal durability.");
    println!("[PUB QoS] Start sub_qos to see retained data.");
    std::thread::sleep(std::time::Duration::from_secs(30));

    Ok(())
}
