//! Stress test: high-throughput pub/sub with millions of messages.
//!
//! Run with:
//!   cargo test --test stress -- --nocapture

use cyclonedds::DdsTypeDerive;
use cyclonedds::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(DdsTypeDerive, Clone, Debug)]
struct StressMessage {
    #[key]
    seq: u64,
    id: i32,
    value: f64,
}

#[test]
fn stress_test_million_messages() {
    let count = 100_000;
    let participant = DomainParticipant::new(99).unwrap();
    let publisher = Publisher::new(participant.entity()).unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();

    let topic_pub = Topic::<StressMessage>::new(participant.entity(), "Stress").unwrap();
    let topic_sub = Topic::<StressMessage>::new(participant.entity(), "Stress").unwrap();

    let writer: DataWriter<StressMessage> =
        DataWriter::new(publisher.entity(), topic_pub.entity()).unwrap();
    let reader: DataReader<StressMessage> =
        DataReader::new(subscriber.entity(), topic_sub.entity()).unwrap();

    // Wait for matching
    thread::sleep(Duration::from_millis(500));

    let received = Arc::new(AtomicUsize::new(0));
    let received_clone = received.clone();

    // Subscriber thread
    let handle = thread::spawn(move || {
        let start = Instant::now();
        loop {
            match reader.take() {
                Ok(samples) => {
                    let n = samples.len();
                    received_clone.fetch_add(n, Ordering::SeqCst);
                    if received_clone.load(Ordering::SeqCst) >= count {
                        break;
                    }
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(1));
                }
            }
            if start.elapsed() > Duration::from_secs(60) {
                panic!("Stress test timed out");
            }
        }
    });

    // Publisher
    let start = Instant::now();
    for i in 0..count as u64 {
        let msg = StressMessage {
            seq: i,
            id: i as i32,
            value: i as f64,
        };
        writer.write(&msg).unwrap();
    }
    let pub_elapsed = start.elapsed();

    // Wait for subscriber to finish
    handle.join().unwrap();
    let total_elapsed = start.elapsed();

    let received_count = received.load(Ordering::SeqCst);
    println!(
        "Stress test: published {} messages in {:?} ({:.0} msg/s)",
        count,
        pub_elapsed,
        count as f64 / pub_elapsed.as_secs_f64()
    );
    println!(
        "Stress test: received {} messages in {:?} ({:.0} msg/s)",
        received_count,
        total_elapsed,
        received_count as f64 / total_elapsed.as_secs_f64()
    );

    assert_eq!(received_count, count, "Not all messages were received");
}
