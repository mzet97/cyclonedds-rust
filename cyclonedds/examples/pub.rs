use cyclonedds::*;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let publisher = Publisher::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "HelloWorldTopic")?;
    let writer = DataWriter::new(publisher.entity(), topic.entity())?;

    let mut msg = HelloWorld {
        id: 0,
        message: [0u8; 256],
    };
    let text = b"Hello from Rust DDS!";
    msg.message[..text.len()].copy_from_slice(text);

    for i in 0..10 {
        msg.id = i;
        writer.write(&msg)?;
        println!("Published: id={}", i);
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(())
}
