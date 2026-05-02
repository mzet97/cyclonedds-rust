//! Shared-memory transport example with Iceoryx/PSMX.
//!
//! This example demonstrates how to enable shared-memory transport
//! between a publisher and subscriber on the same machine using
//! CycloneDDS's PSMX (Platform-Specific Memory eXchange) support.
//!
//! # Prerequisites
//!
//! 1. Build CycloneDDS with Iceoryx support (`-DENABLE_ICEORYX=ON`).
//! 2. Ensure the `iceoryx` PSMX library is available at runtime.
//!
//! # Running
//!
//! Terminal 1:
//!   cargo run --example shm_sub
//!
//! Terminal 2:
//!   cargo run --example shm_pub

use cyclonedds::{
    DataWriter, DdsEntity, DdsTypeDerive, DomainParticipant, Publisher, QosBuilder, Topic,
};

#[derive(DdsTypeDerive, Clone, Debug)]
struct LargeMessage {
    id: i32,
    payload: Vec<u8>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let publisher = Publisher::new(participant.entity())?;

    let qos = QosBuilder::new().enable_iceoryx().build()?;

    let topic = Topic::<LargeMessage>::new(participant.entity(), "LargeData")?;
    let writer = DataWriter::with_qos(publisher.entity(), topic.entity(), Some(&qos))?;

    println!("SHM publisher started. Sending large messages via Iceoryx...");

    let payload = vec![0xABu8; 1024 * 1024]; // 1 MB payload
    for i in 0..100 {
        let msg = LargeMessage {
            id: i,
            payload: payload.clone(),
        };
        writer.write(&msg)?;
        println!("Published message {} ({} bytes)", i, msg.payload.len());
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("Done!");
    Ok(())
}
