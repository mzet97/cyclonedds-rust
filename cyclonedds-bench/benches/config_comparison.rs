use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use cyclonedds::*;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct ConfigMsg {
    id: i32,
    payload: [u8; 256],
}

fn bench_reliability_modes(c: &mut Criterion) {
    let participant = DomainParticipant::new(98).unwrap();
    let topic_name = format!("config_compare_{}", std::process::id());

    let topic_be = participant
        .create_topic_with_qos::<ConfigMsg>(
            &topic_name,
            &QosBuilder::new()
                .reliability(Reliability::BestEffort)
                .build()
                .unwrap(),
        )
        .unwrap();

    let topic_rel = participant
        .create_topic_with_qos::<ConfigMsg>(
            &topic_name,
            &QosBuilder::new()
                .reliability(Reliability::Reliable, 10_000_000_000)
                .build()
                .unwrap(),
        )
        .unwrap();

    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();

    let writer_be = publisher.create_writer(&topic_be).unwrap();
    let reader_be = subscriber.create_reader(&topic_be).unwrap();

    let writer_rel = publisher.create_writer(&topic_rel).unwrap();
    let reader_rel = subscriber.create_reader(&topic_rel).unwrap();

    std::thread::sleep(Duration::from_millis(300));

    let msg = ConfigMsg {
        id: 1,
        payload: [0u8; 256],
    };

    let mut group = c.benchmark_group("config/reliability");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(8));

    group.bench_function(BenchmarkId::new("latency", "best_effort"), |b| {
        b.iter(|| {
            let _ = reader_be.take();
            let start = Instant::now();
            writer_be.write(&msg).unwrap();
            loop {
                if let Ok(samples) = reader_be.read() {
                    if !samples.is_empty() {
                        break start.elapsed();
                    }
                }
                std::thread::sleep(Duration::from_micros(10));
            }
        });
    });

    group.bench_function(BenchmarkId::new("latency", "reliable"), |b| {
        b.iter(|| {
            let _ = reader_rel.take();
            let start = Instant::now();
            writer_rel.write(&msg).unwrap();
            loop {
                if let Ok(samples) = reader_rel.read() {
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

criterion_group!(config_comparison, bench_reliability_modes);
criterion_main!(config_comparison);
