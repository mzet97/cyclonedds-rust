//! Hello World Subscriber Example
//!
//! Run this example with: cargo run --example sub
//! Then run pub example in another terminal to send messages.

use cyclonedds::*;

/// Simple message type for demonstration (must match publisher)
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct HelloWorld {
    id: i32,
    message: String,
}

fn main() {
    println!("Starting CycloneDDS Subscriber...");

    // Create DomainParticipant for domain 0
    let participant = DomainParticipant::new(0)
        .expect("Failed to create DomainParticipant");

    // Create a topic
    let topic: Topic<HelloWorld> = participant
        .create_topic("HelloWorldTopic")
        .expect("Failed to create topic");

    // Create Subscriber
    let subscriber = participant
        .create_subscriber()
        .expect("Failed to create subscriber");

    // Create DataReader
    let reader: DataReader<HelloWorld> = subscriber
        .create_reader(&topic)
        .expect("Failed to create reader");

    println!("Subscriber ready. Waiting for messages...");

    loop {
        // Wait for data with timeout
        match reader.wait(1000) {
            Ok(true) => {
                // Data available, read it
                match reader.read() {
                    Ok(messages) => {
                        for msg in messages {
                            println!("Received: {:?}", msg);
                        }
                    }
                    Err(e) => eprintln!("Failed to read: {:?}", e),
                }
            }
            Ok(false) => {
                // Timeout, continue waiting
            }
            Err(e) => eprintln!("Wait error: {:?}", e),
        }
    }
}
