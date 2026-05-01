use cyclonedds::*;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Clone, Copy)]
struct BenchMsg {
    id: i32,
    payload: [u8; 256],
}

impl BenchMsg {
    fn new(id: i32) -> Self {
        Self { id, payload: [0u8; 256] }
    }
}

impl DdsType for BenchMsg {
    fn type_name() -> &'static str { "BenchMsg" }
    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 0));
        ops.extend(adr_bst(4, 256));
        ops
    }
}

fn main() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic_name = format!("bench_latency_{}", std::process::id());
    let topic = participant.create_topic::<BenchMsg>(&topic_name).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    // Wait for match
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if !writer.matched_subscriptions().unwrap().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    const N: usize = 1000;
    let mut latencies = Vec::with_capacity(N);

    for i in 0..N {
        let msg = BenchMsg::new(i as i32);
        let t0 = Instant::now();
        writer.write(&msg).unwrap();

        // Wait for sample
        loop {
            for _ in reader.take().unwrap() {
                latencies.push(t0.elapsed().as_nanos() as f64 / 1e6); // ms
                break;
            }
            if latencies.len() > i {
                break;
            }
            std::thread::sleep(Duration::from_micros(10));
        }
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = latencies[N / 2];
    let p99 = latencies[N * 99 / 100];
    let min = latencies[0];
    let max = latencies[N - 1];
    let avg: f64 = latencies.iter().sum::<f64>() / latencies.len() as f64;

    println!("=== Latency Benchmark ===");
    println!("Samples: {}", N);
    println!("Min:     {:.3} ms", min);
    println!("Max:     {:.3} ms", max);
    println!("Avg:     {:.3} ms", avg);
    println!("P50:     {:.3} ms", p50);
    println!("P99:     {:.3} ms", p99);
}
