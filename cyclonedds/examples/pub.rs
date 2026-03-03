//! Hello World Publisher Example
//!
//! Run this example with: cargo run --example pub
//! Then run sub example in another terminal to receive messages.

use std::time::Duration;
use cyclonedds::*;

/// Simple message type for demonstration
#[derive(serde::Serialize, serde::Deserialize)]
struct HelloWorld {
    id: i32,
    message: String,
}

fn main() {
    println!("Starting CycloneDDS Publisher...");

    // Create DomainParticipant for domain 0
    let participant = DomainParticipant::new(0)
        .expect("Failed to create DomainParticipant");

    // Create a topic
    let topic: Topic<HelloWorld> = participant
        .create_topic("HelloWorldTopic")
        .expect("Failed to create topic");

    // Create Publisher
    let publisher = participant
        .create_publisher()
        .expect("Failed to create publisher");

    // Create DataWriter
    let writer: DataWriter<HelloWorld> = publisher
        .create_writer(&topic)
        .expect("Failed to create writer");

    println!("Publisher ready. Publishing messages...");

    let mut id = 0;
    loop {
        let msg = HelloWorld {
            id,
            message: format!("Hello, World! #{}", id),
        };

        match writer.write(&msg) {
            Ok(_) => println!("Published: {:?}", msg),
            Err(e) => eprintln!("Failed to write: {:?}", e),
        }

        id += 1;
        std::thread::sleep(Duration::from_secs(1));
    }
}
