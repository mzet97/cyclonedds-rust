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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let subscriber = Subscriber::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "HelloWorldTopic")?;
    let reader: DataReader<HelloWorld> = DataReader::new(subscriber.entity(), topic.entity())?;

    let waitset = WaitSet::new(participant.entity())?;
    let cond = ReadCondition::not_read(reader.entity())?;
    waitset.attach(cond.entity(), 1)?;

    println!("[ASYNC] Waiting for data via WaitSet...");
    loop {
        let triggered = waitset.wait_async(5_000_000_000).await?;
        if triggered.is_empty() {
            println!("[ASYNC] Timeout, waiting again...");
            continue;
        }
        let samples = reader.take_async().await?;
        for s in &samples {
            let end = s.message.iter().position(|&b| b == 0).unwrap_or(256);
            let text = std::str::from_utf8(&s.message[..end]).unwrap_or("?");
            println!("[ASYNC] id={}, message={}", s.id, text);
        }
    }
}
