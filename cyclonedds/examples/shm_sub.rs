//! Shared-memory transport subscriber example with Iceoryx/PSMX.
//!
//! Run with:
//!   cargo run --example shm_sub

use cyclonedds::{DomainParticipant, Subscriber, QosBuilder, Topic, DataReader, DdsEntity, DdsTypeDerive};

#[derive(DdsTypeDerive, Clone, Debug)]
struct LargeMessage {
    id: i32,
    payload: Vec<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let subscriber = Subscriber::new(participant.entity())?;

    let qos = QosBuilder::new()
        .enable_iceoryx()
        .build()?;

    let topic = Topic::<LargeMessage>::new(participant.entity(), "LargeData")?;
    let reader: DataReader<LargeMessage> = DataReader::with_qos(subscriber.entity(), topic.entity(), Some(&qos))?;

    println!("SHM subscriber started. Waiting for large messages via Iceoryx...");

    let mut received = 0;
    loop {
        for msg in reader.take()? {
            received += 1;
            println!("Received message {} ({} bytes)", msg.id, msg.payload.len());
            if received >= 100 {
                println!("Received all 100 messages.");
                return Ok(());
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
