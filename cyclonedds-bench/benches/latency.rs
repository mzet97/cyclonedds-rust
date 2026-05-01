use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use cyclonedds::*;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct Message64 {
    id: i32,
    payload: [u8; 64],
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct Message1K {
    id: i32,
    payload: [u8; 1024],
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct Message16K {
    id: i32,
    payload: [u8; 16384],
}

fn measure_roundtrip<T: DdsType + Clone>(c: &mut Criterion, name: &str, mut make_msg: impl FnMut(i32) -> T) {
    let participant = DomainParticipant::new(99).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<T>(&format!("latency_{}_{}", name, std::process::id()))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    // Wait for discovery
    std::thread::sleep(Duration::from_millis(200));

    let mut group = c.benchmark_group(format!("latency/{}", name));
    group.sample_size(50);
    group.measurement_time(Duration::from_secs(10));

    group.bench_function(BenchmarkId::new("reliable", name), |b| {
        let mut seq = 0;
        b.iter(|| {
            seq += 1;
            let msg = make_msg(seq);
            // Drain any stale samples
            let _ = reader.take();
            let start = Instant::now();
            writer.write(&msg).unwrap();
            loop {
                if let Ok(samples) = reader.read() {
                    if !samples.is_empty() {
                        break start.elapsed();
                    }
                }
                std::thread::sleep(Duration::from_micros(10));
            }
        });
    });

    group.finish();
}

fn latency_64b(c: &mut Criterion) {
    measure_roundtrip(c, "64b", |id| Message64 {
        id,
        payload: [0u8; 64],
    });
}

fn latency_1kb(c: &mut Criterion) {
    measure_roundtrip(c, "1kb", |id| Message1K {
        id,
        payload: [0u8; 1024],
    });
}

fn latency_16kb(c: &mut Criterion) {
    measure_roundtrip(c, "16kb", |id| Message16K {
        id,
        payload: [0u8; 16384],
    });
}

criterion_group!(latency, latency_64b, latency_1kb, latency_16kb);
criterion_main!(latency);
