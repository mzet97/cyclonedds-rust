//! What `take_async` costs over `take`.
//!
//! D6 asked for a before/after on dropping `spawn_blocking` from the read path.
//! That could not be answered with the benchmarks this crate had: `latency`,
//! `throughput`, `cdr` and `config_comparison` are entirely synchronous, and
//! `ipc_comparison` mentions async only in a comment. Nothing measured
//! `take_async`, so the claim that removing `spawn_blocking` cut latency had
//! nothing behind it.
//!
//! This measures the pair on identical work. `dds_take` does not block — it
//! walks the reader history cache and returns — so `take_async` costs what
//! `take` costs plus the future's own overhead. When it was wrapped in
//! `spawn_blocking` it also paid a thread hop and a scheduler round trip on
//! every call.
//!
//! Measured on x86_64-pc-windows-msvc, rustc 1.85.0, current-thread runtime, by
//! reintroducing the `spawn_blocking` wrapper in `take_async` and re-running:
//!
//! | | `take/sync` | `take/async` |
//! |---|---|---|
//! | with `spawn_blocking` (pre-2.0.4) | 814 ns | **18.38 µs** |
//! | inline (current) | 831 ns | **1.016 µs** |
//!
//! So the claim D6 recorded as unmeasured holds and is now a number: **~18×**,
//! about 17.4 µs per call. The synchronous arm is the control and does not move.
//!
//! Read this as the current baseline, not as a target: it is one machine, one
//! runtime flavour, and a current-thread runtime is the shape where a
//! `spawn_blocking` hop costs most.
//!
//! ```bash
//! cargo bench -p cyclonedds-bench --bench async_read --features async
//! ```

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cyclonedds::*;

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct AsyncMsg {
    id: i32,
    payload: [u8; 64],
}

struct Fixture {
    _participant: DomainParticipant,
    _publisher: Publisher,
    _subscriber: Subscriber,
    _topic: Topic<AsyncMsg>,
    writer: DataWriter<AsyncMsg>,
    reader: DataReader<AsyncMsg>,
}

fn fixture(domain: u32, name: &str) -> Fixture {
    let participant = DomainParticipant::new(domain).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<AsyncMsg>(&format!("{name}_{}", std::process::id()))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));

    Fixture {
        _participant: participant,
        _publisher: publisher,
        _subscriber: subscriber,
        _topic: topic,
        writer,
        reader,
    }
}

fn sample(id: i32) -> AsyncMsg {
    AsyncMsg {
        id,
        payload: [0u8; 64],
    }
}

fn bench_take_sync(c: &mut Criterion) {
    let f = fixture(97, "async_read_sync");

    c.bench_function("take/sync", |b| {
        b.iter(|| {
            f.writer.write(&sample(1)).unwrap();
            black_box(f.reader.take().unwrap());
        })
    });
}

#[cfg(feature = "async")]
fn bench_take_async(c: &mut Criterion) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let f = fixture(97, "async_read_async");

    c.bench_function("take/async", |b| {
        b.iter(|| {
            f.writer.write(&sample(1)).unwrap();
            black_box(runtime.block_on(f.reader.take_async()).unwrap());
        })
    });
}

#[cfg(not(feature = "async"))]
fn bench_take_async(_c: &mut Criterion) {
    eprintln!("take/async skipped: build with --features cyclonedds/async");
}

criterion_group!(benches, bench_take_sync, bench_take_async);
criterion_main!(benches);
