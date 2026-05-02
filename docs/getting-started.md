# Getting Started

Prerequisites: **Rust 1.85+**, **CMake 3.10+**, C/C++ compiler. Clang is only needed for maintainers regenerating FFI bindings.

By default `cyclonedds-sys` builds the bundled CycloneDDS source from `vendor/`, so no system-level install is needed. Override with `CYCLONEDDS_SRC` or `CYCLONEDDS_BUILD`.

### Installing CycloneDDS (optional)

Linux: `sudo apt install cyclonedds-dev` | macOS: `brew install cyclonedds` | Windows: use vcpkg or bundled vendor.

## Add to Your Project

```toml
[dependencies]
cyclonedds = "1.5"  # async is enabled by default
```

## Define a Topic Type

```rust
use cyclonedds::*;

#[repr(C)]
struct HelloWorld { id: i32, message: [u8; 256] }

impl DdsType for HelloWorld {
    fn type_name() -> &'static str { "HelloWorld" }
    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 0));
        ops.extend(adr_bst(4, 256));
        ops
    }
}
```

For the derive macro approach, see [type-system.md](type-system.md).

## Publisher

```rust
use cyclonedds::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let publisher = Publisher::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "HelloWorldTopic")?;
    let writer = DataWriter::new(publisher.entity(), topic.entity())?;
    let mut msg = HelloWorld { id: 0, message: [0u8; 256] };
    let text = b"Hello from Rust DDS!";
    msg.message[..text.len()].copy_from_slice(text);
    for i in 0..10 {
        msg.id = i;
        writer.write(&msg)?;
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    Ok(())
}
```

## Subscriber

```rust
use cyclonedds::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let subscriber = Subscriber::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "HelloWorldTopic")?;
    let reader = DataReader::<HelloWorld>::new(subscriber.entity(), topic.entity())?;
    loop {
        for s in reader.take()? {
            let end = s.message.iter().position(|&b| b == 0).unwrap_or(256);
            let text = std::str::from_utf8(&s.message[..end]).unwrap_or("?");
            println!("id={}, message={}", s.id, text);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
```

## Run the Examples

The repository includes ready-to-run examples:

```bash
# Terminal 1 -- subscriber
cargo run --example sub

# Terminal 2 -- publisher
cargo run --example pub
```

## Configuration

CycloneDDS reads its configuration from an XML file. Set the path via the `CYCLONEDDS_URI` environment variable:

```bash
export CYCLONEDDS_URI=file://$(pwd)/cyclonedds.xml
cargo run --example pub
```

The repository ships example configs (`cyclonedds.xml`) for local and network operation.

## Async Streams

With the `async` feature (enabled by default), `DataReader` can produce an async stream:

```rust
use cyclonedds::DataReader;
use futures_util::StreamExt;

# async fn example<T: cyclonedds::DdsType>(reader: &DataReader<T>) {
let mut stream = Box::pin(reader.read_aiter());
while let Some(batch) = stream.next().await {
    match batch {
        Ok(samples) => println!("got {} samples", samples.len()),
        Err(e) => eprintln!("read error: {}", e),
    }
}
# }
```

## WSL Build Notes

When building in WSL, `cyclonedds-sys` compiles the bundled CycloneDDS C library. After the first successful build, ensure the shared library is discoverable:

```bash
export LD_LIBRARY_PATH=~/cyclonedds-rust/vendor/cyclonedds/build/lib:$LD_LIBRARY_PATH
cargo test --workspace --features async
```

## Next Steps

- [API Guide](api-guide.md) -- tour of all major API features
- [Type System](type-system.md) -- DdsType derive, supported types, CDR encoding
- [QoS Reference](qos-reference.md) -- all QoS policies and builder patterns
