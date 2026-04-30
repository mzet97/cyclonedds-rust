# Migrating from cyclonedds-python

This guide helps developers familiar with `cyclonedds-python` transition to `cyclonedds-rust`.

## High-level Differences

| Aspect | Python | Rust |
|--------|--------|------|
| Type safety | Runtime | Compile-time |
| Memory management | GC | Ownership + RAII |
| Serialization | Automatic (Cython) | Derive macro or manual `DdsType` impl |
| Async model | `asyncio` + custom waitset | `tokio` + native `Stream` impl |
| Error handling | Exceptions | `Result<T, DdsError>` |

## Entity Mapping

| Python | Rust |
|--------|------|
| `DomainParticipant(domain_id)` | `DomainParticipant::new(domain_id)?` |
| `Publisher(participant)` | `Publisher::new(participant.entity())?` |
| `Subscriber(participant)` | `Subscriber::new(participant.entity())?` |
| `Topic(participant, "Name", T)` | `Topic::<T>::new(participant.entity(), "Name")?` |
| `DataWriter(publisher, topic)` | `DataWriter::new(publisher.entity(), topic.entity())?` |
| `DataReader(subscriber, topic)` | `DataReader::<T>::new(subscriber.entity(), topic.entity())?` |

## QoS

Python uses a fluent builder:

```python
from cyclonedds.qos import Qos, Policy
qos = Qos(Policy.Reliability.BestEffort)
```

Rust uses `QosBuilder`:

```rust
use cyclonedds::QosBuilder;
let qos = QosBuilder::new().reliable().build();
```

## Reading Data

Python:

```python
samples = reader.take()
for s in samples:
    print(s.data)
```

Rust:

```rust
let samples: Vec<T> = reader.take()?;
for s in samples {
    println!("{:?}", s);
}
```

Note: Rust's `take()` returns `Vec<T>` directly, not wrapped in `Sample` objects. If you need metadata (timestamp, instance state), use `reader.read_with_metadata()` or similar advanced APIs.

## Async Iterators

Rust provides native `Stream` support (Python does not have an equivalent):

```rust
use futures_util::StreamExt;

let mut stream = Box::pin(reader.read_aiter());
while let Some(batch) = stream.next().await {
    match batch {
        Ok(samples) => println!("got {} samples", samples.len()),
        Err(e) => eprintln!("read error: {}", e),
    }
}
```

## Content-Filtered Topics

Both Python and Rust use closure-based filtering (CycloneDDS C does not expose SQL content filters):

**Python:**
```python
from cyclonedds.topic import ContentFilteredTopic
cft = ContentFilteredTopic(topic, lambda s: s.id > 10)
```

**Rust:**
```rust
let cft = ContentFilteredTopic::new(&topic, |s: &T| s.id > 10)?;
```

## CLI Tools

| Python | Rust |
|--------|------|
| `cyclonedds ls` | `cargo run --bin cyclonedds-cli -- ls` |
| `cyclonedds ps` | `cargo run --bin cyclonedds-cli -- ps` |
| `cyclonedds subscribe` | `cargo run --bin cyclonedds-cli -- subscribe` |
| `cyclonedds typeof` | `cargo run --bin cyclonedds-cli -- typeof` |
| `cyclonedds publish` | `cargo run --bin cyclonedds-cli -- publish` |

## Common Pitfalls

1. **Entity handles**: Rust uses `entity()` to get the raw handle; Python entities are handles directly.
2. **Lifetime**: Rust `Topic`, `DataReader`, `DataWriter` are bound to their parent participant's lifetime. Dropping the participant invalidates children.
3. **Type registration**: In Rust, `Topic::<T>` requires `T: DdsType`. Use `#[derive(DdsType)]` or implement the trait manually.
4. **No `Sample<T>` wrapper**: Unlike Python, Rust `read()` / `take()` return `Vec<T>` directly.

## Feature Parity

~96% of cyclonedds-python APIs are available in Rust. Notable gaps:

- **SQL Content Filters**: Neither Python nor Rust supports SQL syntax; both use closures.
- **QueryCondition SQL**: Same as above — C API limitation.
- **QoS merge/diff**: Not exposed in Python or Rust.

See the [API Guide](api-guide.md) for a complete tour of Rust-specific features like zero-copy loans and matched endpoint data.
