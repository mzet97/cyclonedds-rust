use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use cyclonedds::*;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct ThroughputMessage {
    id: i32,
    payload: [u8; 1024],
}

fn throughput_benchmark(c: &mut Criterion) {
    let participant = DomainParticipant::new(99).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<ThroughputMessage>(&format!("throughput_{}", std::process::id()))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    std::thread::sleep(Duration::from_millis(200));

    let mut group = c.benchmark_group("throughput");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));

    for batch_size in [1, 10, 100] {
        group.bench_with_input(
            BenchmarkId::new("msg_per_sec", batch_size),
            &batch_size,
            |b, &batch| {
                let msg = ThroughputMessage {
                    id: 1,
                    payload: [0u8; 1024],
                };
                let total_messages: usize = 1000;

                b.iter(|| {
                    // Drain reader before run
                    while let Ok(samples) = reader.take() {
                        if samples.is_empty() {
                            break;
                        }
                    }

                    let start = Instant::now();

                    // Publish messages in batches
                    for i in 0..total_messages {
                        let mut m = msg.clone();
                        m.id = i as i32;
                        writer.write(&m).unwrap();
                        if batch > 1 && i % batch == 0 {
                            writer.write_flush().unwrap();
                        }
                    }
                    writer.write_flush().unwrap();

                    // Wait for all messages to arrive
                    let timeout = Duration::from_secs(30);
                    let mut received = 0;
                    loop {
                        if let Ok(samples) = reader.take() {
                            received += samples.len();
                        }
                        if received >= total_messages {
                            break;
                        }
                        if start.elapsed() > timeout {
                            break;
                        }
                        std::thread::sleep(Duration::from_micros(100));
                    }

                    let elapsed = start.elapsed();
                    // Return throughput as msg/s (criterion will use the throughput custom measurement if configured)
                    // For criterion, we just return the elapsed time; throughput can be calculated from total_messages / elapsed
                    elapsed / total_messages as u32
                });
            },
        );
    }

    group.finish();
}

criterion_group!(throughput, throughput_benchmark);
criterion_main!(throughput);
