# Tutorial: Your First DDS Application in Rust

This tutorial walks you through building a complete DDS pub/sub application using `cyclonedds-rust`.

## Prerequisites

- Rust 1.85+ installed
- CMake 3.10+ installed
- A C/C++ compiler

## Step 1: Create a New Project

```bash
cargo new dds_hello_world
cd dds_hello_world
```

## Step 2: Add Dependencies

Edit `Cargo.toml`:

```toml
[dependencies]
cyclonedds = "1.4"
cyclonedds-derive = "1.4"
```

## Step 3: Define Your Topic Type

Create `src/main.rs`:

```rust
use cyclonedds::{DomainParticipant, Publisher, Subscriber, Topic, DataWriter, DataReader, DdsTypeDerive, DdsEntity};
use std::thread;
use std::time::Duration;

#[derive(DdsTypeDerive, Clone, Debug)]
struct HelloWorld {
    id: i32,
    message: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // We'll run both pub and sub in the same process for this demo.
    // In a real app, you'd run them in separate processes.

    let participant = DomainParticipant::new(0)?;

    // --- Publisher side ---
    let publisher = Publisher::new(participant.entity())?;
    let topic_pub = Topic::<HelloWorld>::new(participant.entity(), "HelloWorld")?;
    let writer = DataWriter::new(publisher.entity(), topic_pub.entity())?;

    // --- Subscriber side ---
    let subscriber = Subscriber::new(participant.entity())?;
    let topic_sub = Topic::<HelloWorld>::new(participant.entity(), "HelloWorld")?;
    let reader = DataReader::new(subscriber.entity(), topic_sub.entity())?;

    // Spawn publisher thread
    std::thread::spawn(move || {
        for i in 0..10 {
            let msg = HelloWorld {
                id: i,
                message: format!("Hello from DDS! #{}", i),
            };
            writer.write(&msg).unwrap();
            println!("Published: {:?}", msg);
            thread::sleep(Duration::from_millis(500));
        }
    });

    // Read samples
    loop {
        for sample in reader.take()? {
            println!("Received: {:?}", sample);
        }
        thread::sleep(Duration::from_millis(100));
    }
}
```

## Step 4: Build and Run

```bash
cargo run
```

You should see published and received messages interleaved.

## Step 5: Add QoS

Improve reliability with QoS:

```rust
use cyclonedds::QosBuilder;

let qos = QosBuilder::new()
    .reliability(cyclonedds::Reliability::Reliable)
    .history(cyclonedds::History::KeepLast(10))
    .build()?;

let writer = DataWriter::with_qos(publisher.entity(), topic_pub.entity(), Some(&qos))?;
```

## Step 6: Run Separate Processes

Split into two binaries:

**`src/bin/pub.rs`**:
```rust
use cyclonedds::*;
// ... publisher code only, sleep at end
```

**`src/bin/sub.rs`**:
```rust
use cyclonedds::*;
// ... subscriber code only, loop forever
```

Run in two terminals:
```bash
cargo run --bin pub
cargo run --bin sub
```

## Next Steps

- Read the [API Guide](api-guide.md) for all features.
- Explore [QoS Reference](qos-reference.md) for advanced policies.
- Check [ROS2 Integration](ros2-integration.md) to connect with ROS2 nodes.
