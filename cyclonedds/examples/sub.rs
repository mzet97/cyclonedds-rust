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
    let subscriber = Subscriber::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "HelloWorldTopic")?;
    let reader: DataReader<HelloWorld> = DataReader::new(subscriber.entity(), topic.entity())?;

    println!("Waiting for samples...");
    loop {
        let samples = reader.take()?;
        for s in &samples {
            let end = s.message.iter().position(|&b| b == 0).unwrap_or(256);
            let text = std::str::from_utf8(&s.message[..end]).unwrap_or("?");
            println!("Received: id={}, message={}", s.id, text);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
