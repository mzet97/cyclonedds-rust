use cyclonedds::*;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Clone, Copy)]
struct TestMessage {
    id: i32,
    value: i32,
    text: [u8; 64],
}

impl TestMessage {
    fn new(id: i32, value: i32, text: &str) -> Self {
        let mut bytes = [0u8; 64];
        let encoded = text.as_bytes();
        let len = encoded.len().min(bytes.len().saturating_sub(1));
        bytes[..len].copy_from_slice(&encoded[..len]);
        Self {
            id,
            value,
            text: bytes,
        }
    }
}

impl DdsType for TestMessage {
    fn type_name() -> &'static str {
        "InteropTestMessage"
    }
    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 0));
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 4));
        ops.extend(adr_bst(8, 64));
        ops
    }
}

fn main() {
    let domain_id: u32 = std::env::var("DDS_DOMAIN_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let topic_name = std::env::var("DDS_TOPIC_NAME").unwrap_or_else(|_| "interop_test".to_string());
    let expected_count: i32 = std::env::var("DDS_PUB_COUNT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    let participant = DomainParticipant::new(domain_id).unwrap();
    let topic = participant
        .create_topic::<TestMessage>(&topic_name)
        .unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    let start = Instant::now();
    let timeout = Duration::from_secs(30);
    let mut received = 0;

    loop {
        if start.elapsed() > timeout {
            eprintln!("TIMEOUT");
            std::process::exit(1);
        }

        for sample in reader.take().unwrap() {
            if sample.id == -1 {
                // End marker
                if received == expected_count {
                    println!("OK: received all {} samples", received);
                    std::process::exit(0);
                } else {
                    eprintln!("MISMATCH: expected {}, got {}", expected_count, received);
                    std::process::exit(1);
                }
            }
            received += 1;
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}
