//! DDS Statistics example — collect and print entity metrics.
//!
//! This example demonstrates how to use the `Statistics` API to query
//! runtime metrics from a DDS entity (participant, reader, writer).
//!
//! Run with:
//!   cargo run --example metrics

use cyclonedds::{DomainParticipant, Publisher, Topic, DataWriter, DdsEntity, DdsTypeDerive};

#[derive(DdsTypeDerive, Clone, Debug)]
struct HelloWorld {
    id: i32,
    message: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let publisher = Publisher::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "Hello")?;
    let writer = DataWriter::new(publisher.entity(), topic.entity())?;

    // Publish a few samples
    for i in 0..5 {
        let msg = HelloWorld {
            id: i,
            message: format!("Hello {}", i),
        };
        writer.write(&msg)?;
    }

    // Query statistics from the writer
    let stats = writer.create_statistics()?;
    println!("Writer statistics ({} entries):", stats.len());
    for entry in stats.entries() {
        println!("  {} = {:?}", entry.name(), entry.value());
    }

    // Look up a specific statistic by name
    if let Some(entry) = stats.lookup("heartbeat_count")? {
        println!("heartbeat_count = {:?}", entry.value());
    }

    // Query participant-level statistics
    let pstats = participant.create_statistics()?;
    println!("\nParticipant statistics ({} entries):", pstats.len());
    for entry in pstats.entries() {
        println!("  {} = {:?}", entry.name(), entry.value());
    }

    Ok(())
}
