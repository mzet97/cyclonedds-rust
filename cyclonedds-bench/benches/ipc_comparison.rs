use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cyclonedds::{
    DataReader, DataWriter, DdsEntity, DdsTypeDerive, DomainParticipant, Publisher, Subscriber,
    Topic,
};
use std::thread;
use std::time::Duration;

#[derive(DdsTypeDerive, Clone, Debug)]
#[repr(C)]
struct PingPong {
    seq: u64,
    payload: [u8; 64],
}

fn bench_dds_latency(c: &mut Criterion) {
    // This benchmark measures round-trip latency in the same process.
    // For true IPC comparison, run with --features async and use separate processes.
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();

    let topic_pub = Topic::<PingPong>::new(participant.entity(), "PingPong").unwrap();
    let topic_sub = Topic::<PingPong>::new(participant.entity(), "PingPong").unwrap();

    let writer: DataWriter<PingPong> =
        DataWriter::new(publisher.entity(), topic_pub.entity()).unwrap();
    let reader: DataReader<PingPong> =
        DataReader::new(subscriber.entity(), topic_sub.entity()).unwrap();

    thread::sleep(Duration::from_millis(200));

    let msg = PingPong {
        seq: 0,
        payload: [0u8; 64],
    };

    c.bench_function("dds_roundtrip_latency_64b", |b| {
        b.iter(|| {
            writer.write(black_box(&msg)).unwrap();
            loop {
                match reader.take() {
                    Ok(samples) if !samples.is_empty() => break,
                    _ => thread::sleep(Duration::from_micros(10)),
                }
            }
        })
    });
}

fn bench_std_channel_latency(c: &mut Criterion) {
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel::<PingPong>();
    let msg = PingPong {
        seq: 0,
        payload: [0u8; 64],
    };

    c.bench_function("std_channel_roundtrip_latency_64b", |b| {
        b.iter(|| {
            tx.send(black_box(msg.clone())).unwrap();
            let _ = rx.recv().unwrap();
        })
    });
}

criterion_group!(benches, bench_dds_latency, bench_std_channel_latency,);
criterion_main!(benches);
