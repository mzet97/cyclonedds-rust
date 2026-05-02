# API Guide

A tour of the cyclonedds-rust API with short code snippets for each major feature area.

## DomainParticipant

The entry point for any DDS application. Represents membership in a DDS domain.

```rust
use cyclonedds::DomainParticipant;

let participant = DomainParticipant::new(0)?; // domain 0
```

From the participant you create Publishers, Subscribers, and Topics. Access the underlying entity handle with `participant.entity()`.

## Topic

A Topic associates a type with a name. The type must implement the `DdsType` trait.

```rust
use cyclonedds::{Topic, DdsType, adr, TYPE_4BY, OP_FLAG_SGN};

let topic: Topic<MyData> = Topic::new(participant.entity(), "MyTopic")?;
```

See [type-system.md](type-system.md) for how to implement `DdsType` manually or via derive macros.

## DataWriter

Publishes typed samples. Create one from a Publisher and a Topic.

```rust
use cyclonedds::{Publisher, DataWriter};

let publisher = Publisher::new(participant.entity())?;
let writer: DataWriter<MyData> = DataWriter::new(publisher.entity(), topic.entity())?;

writer.write(&sample)?;                  // publish
writer.write_dispose(&sample)?;          // publish + dispose
writer.write_cdr(&cdr_bytes)?;           // publish pre-serialized CDR
```

### Zero-copy Loan

Request a loaned buffer from DDS, populate it, and write without copying:

```rust
let mut loan = writer.request_loan()?;
loan.get_mut().id = 42;
loan.get_mut().value = 99;
WriteLoan::write(loan)?;                 // transfers ownership to DDS
```

## DataReader

Receives typed samples. Create one from a Subscriber and a Topic.

```rust
use cyclonedds::{Subscriber, DataReader};

let subscriber = Subscriber::new(participant.entity())?;
let reader: DataReader<MyData> = DataReader::new(subscriber.entity(), topic.entity())?;

let samples: Vec<MyData> = reader.take()?;  // removes from cache
let samples: Vec<MyData> = reader.read()?;  // keeps in cache
```

### Zero-copy Loan

Access sample data without copying (loan is returned on drop):

```rust
let loan = reader.take_loan()?;
for (data, info) in loan.iter() {
    println!("id={}, valid={}", data.id, info.valid_data);
}
```

### Raw CDR

Read/take samples as raw CDR bytes for interoperability or custom deserialization:

```rust
let cdr_samples = reader.read_cdr()?;
let cdr_samples = reader.take_cdr()?;
```

## QoS Policies

Use `QosBuilder` to construct a QoS object. Each method returns `Self` for chaining.

```rust
use cyclonedds::{QosBuilder, Reliability, Durability, History, Ownership};

// Reliable, transient-local, keep-last 10
let qos = QosBuilder::new()
    .reliable()
    .transient_local()
    .keep_last(10)
    .build()?;

// Exclusive ownership with strength 100
let qos = QosBuilder::new()
    .ownership(Ownership::Exclusive)
    .ownership_strength(100)
    .build()?;
```

Pass QoS when creating entities:

```rust
let writer = DataWriter::with_qos(publisher.entity(), topic.entity(), Some(&qos))?;
let reader = DataReader::with_qos(subscriber.entity(), topic.entity(), Some(&qos))?;
```

See [qos-reference.md](qos-reference.md) for the full policy list.

## Listeners

Listeners provide event-driven callbacks. Build one with `ListenerBuilder`:

```rust
use cyclonedds::ListenerBuilder;

let listener = ListenerBuilder::new()
    .on_data_available(|reader_entity| {
        println!("Data available on reader {}", reader_entity);
    })
    .on_publication_matched(|writer_entity, status| {
        println!("Matched {} subscriptions", status.current_count);
    })
    .build()?;
```

Available callbacks: `on_data_available`, `on_publication_matched`, `on_subscription_matched`, `on_liveliness_changed`, `on_inconsistent_topic`, `on_liveliness_lost`, `on_offered_deadline_missed`, `on_offered_incompatible_qos`, `on_data_on_readers`, `on_sample_lost`, `on_sample_rejected`, `on_requested_deadline_missed`, `on_requested_incompatible_qos`.

Pass a listener when creating entities:

```rust
let reader = DataReader::with_listener(subscriber.entity(), topic.entity(), &listener)?;
```

## WaitSet

A WaitSet blocks until one or more attached conditions trigger.

```rust
use cyclonedds::{WaitSet, ReadCondition, GuardCondition};

let waitset = WaitSet::new(participant.entity())?;
let cond = ReadCondition::not_read(reader.entity())?;
let guard = GuardCondition::new(participant.entity())?;

waitset.attach(cond.entity(), 1)?;
waitset.attach(guard.entity(), 2)?;

loop {
    let triggered = waitset.wait(1_000_000_000)?; // 1 second timeout
    for cookie in triggered {
        match cookie {
            1 => { /* data available */ let _ = reader.take(); }
            2 => { /* guard triggered */ }
            _ => {}
        }
    }
}
```

### QueryCondition

Filter samples with a Rust closure:

```rust
use cyclonedds::QueryCondition;

let qc = QueryCondition::with_filter(reader.entity(), 0, |sample_ptr| {
    // Return true to include the sample
    true
})?;
```

## Status API

The `StatusExt` trait provides typed status getters on any entity. It is implemented for all `DdsEntity` types via a blanket impl.

```rust
use cyclonedds::StatusExt;

let status = writer.publication_matched_status()?;
println!("Matched: total={}, current={}", status.total_count, status.current_count);

let status = reader.subscription_matched_status()?;
println!("Subscriptions matched: {}", status.current_count);

let status = reader.liveliness_changed_status()?;
println!("Alive writers: {}", status.alive_count);
```

Available: `inconsistent_topic_status`, `liveliness_lost_status`, `liveliness_changed_status`, `offered_deadline_missed_status`, `offered_incompatible_qos_status`, `requested_deadline_missed_status`, `requested_incompatible_qos_status`, `sample_lost_status`, `sample_rejected_status`, `publication_matched_status`, `subscription_matched_status`.

## CDR Serialization

Serialize and deserialize DDS samples to/from CDR byte streams.

```rust
use cyclonedds::{CdrSerializer, CdrDeserializer, CdrEncoding};

// Serialize
let bytes = CdrSerializer::<MyData>::serialize(&sample, CdrEncoding::Xcdr1)?;

// Deserialize
let sample = CdrDeserializer::<MyData>::deserialize(&bytes, CdrEncoding::Xcdr2)?;
```

## Dynamic Types

Build types at runtime without compile-time struct definitions.

```rust
use cyclonedds::{DynamicTypeBuilder, DynamicTypeExtensibility, DynamicData, DynamicValue};

let mut builder = DynamicTypeBuilder::new(64, 4, DynamicTypeExtensibility::Appendable);
builder.add_member("id", DynamicPrimitiveKind::Int32);
builder.add_member("name", DynamicPrimitiveKind::String);
let dyn_type = builder.build();
```

## Async Support (tokio)

Enable the `async` feature (on by default) to use async/await with DDS operations.

```toml
[dependencies]
cyclonedds = "1.7"  # async feature enabled by default
tokio = { version = "1", features = ["full"] }
```

```rust
use cyclonedds::{DataReader, WaitSet};

// Async wait on a WaitSet
let triggered = waitset.wait_async(5_000_000_000).await?;

// Async take on a DataReader
let samples = reader.take_async().await?;
```

The async methods offload blocking DDS calls to tokio's thread pool via `spawn_blocking`.
