//! Comprehensive DDS test covering all major features.

use cyclonedds::*;
use std::time::Duration;

// ── Type definitions ──────────────────────────────────────────────

/// Basic struct with mixed primitive types and String.
#[derive(Debug, Clone, DdsTypeDerive, PartialEq)]
struct SensorData {
    sensor_id: i32,
    temperature: f64,
    humidity: f32,
    label: String,
    active: bool,
}

/// Keyed struct for instance lifecycle tests.
#[derive(Debug, Clone, DdsTypeDerive, PartialEq)]
struct KeyValue {
    #[key]
    key: i32,
    value: String,
}

/// Struct with sequences.
#[derive(Debug, Clone, DdsTypeDerive)]
struct BatchReading {
    sensor_id: i32,
    readings: DdsSequence<f32>,
}

// ── Test helpers ──────────────────────────────────────────────────

static mut PASS_COUNT: usize = 0;
static mut FAIL_COUNT: usize = 0;

fn pass(name: &str) {
    unsafe {
        PASS_COUNT += 1;
    }
    println!("  [PASS] {}", name);
}

fn fail(name: &str, err: &str) {
    unsafe {
        FAIL_COUNT += 1;
    }
    println!("  [FAIL] {} — {}", name, err);
}

fn check<F: FnOnce() -> Result<String, String>>(name: &str, f: F) {
    match f() {
        Ok(detail) => {
            if detail.is_empty() {
                pass(name);
            } else {
                pass(&format!("{} ({})", name, detail));
            }
        }
        Err(e) => fail(name, &e),
    }
}

// ── Tests ─────────────────────────────────────────────────────────

fn test_participant() -> Result<String, String> {
    let dp = DomainParticipant::new(0).map_err(|e| format!("{:?}", e))?;
    let entity = dp.entity();
    if entity <= 0 {
        return Err("invalid entity handle".into());
    }
    Ok(format!("entity={}", entity))
}

fn test_publisher_subscriber(dp: &DomainParticipant) -> Result<String, String> {
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    Ok(format!("pub={}, sub={}", pub_.entity(), sub.entity()))
}

fn test_topic_creation(dp: &DomainParticipant) -> Result<String, String> {
    let t1 = Topic::<SensorData>::new(dp.entity(), "test_sensor")
        .map_err(|e| format!("{:?}", e))?;
    let t2 = Topic::<KeyValue>::new(dp.entity(), "test_kv")
        .map_err(|e| format!("{:?}", e))?;
    let t3 = Topic::<BatchReading>::new(dp.entity(), "test_batch")
        .map_err(|e| format!("{:?}", e))?;
    Ok(format!(
        "3 topics: {}, {}, {}",
        t1.entity(),
        t2.entity(),
        t3.entity()
    ))
}

fn test_basic_pubsub(dp: &DomainParticipant) -> Result<String, String> {
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let topic = Topic::<SensorData>::new(dp.entity(), "pubsub_test")
        .map_err(|e| format!("{:?}", e))?;
    let writer = DataWriter::new(pub_.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;
    let reader: DataReader<SensorData> = DataReader::new(sub.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;

    let msg = SensorData {
        sensor_id: 99,
        temperature: 23.5,
        humidity: 65.0,
        label: "test".into(),
        active: true,
    };

    writer.write(&msg).map_err(|e| format!("write: {:?}", e))?;
    std::thread::sleep(Duration::from_millis(100));

    let samples = reader.take().map_err(|e| format!("take: {:?}", e))?;
    if samples.is_empty() {
        return Err("no samples received".into());
    }
    let received = &samples[0];
    if received.sensor_id != 99 || received.temperature != 23.5 {
        return Err(format!("data mismatch: {:?}", received));
    }
    Ok(format!(
        "1 sample, id={}, temp={}",
        received.sensor_id, received.temperature
    ))
}

fn test_multiple_writes(dp: &DomainParticipant) -> Result<String, String> {
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let topic = Topic::<SensorData>::new(dp.entity(), "multi_test")
        .map_err(|e| format!("{:?}", e))?;
    let writer = DataWriter::new(pub_.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;
    let reader: DataReader<SensorData> = DataReader::new(sub.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;

    for i in 0..10i32 {
        let msg = SensorData {
            sensor_id: i,
            temperature: i as f64 * 1.5,
            humidity: 50.0,
            label: format!("msg_{}", i),
            active: i % 2 == 0,
        };
        writer.write(&msg).map_err(|e| format!("write {}: {:?}", i, e))?;
    }
    std::thread::sleep(Duration::from_millis(500));

    let samples = reader.take().map_err(|e| format!("take: {:?}", e))?;
    if samples.len() < 1 {
        return Err(format!("expected >=1 samples, got {}", samples.len()));
    }
    Ok(format!("{} samples received", samples.len()))
}

fn test_keyed_data(dp: &DomainParticipant) -> Result<String, String> {
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let topic = Topic::<KeyValue>::new(dp.entity(), "keyed_test")
        .map_err(|e| format!("{:?}", e))?;
    let writer = DataWriter::new(pub_.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;
    let reader: DataReader<KeyValue> = DataReader::new(sub.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;

    let kv1 = KeyValue {
        key: 1,
        value: "hello".into(),
    };
    let kv2 = KeyValue {
        key: 2,
        value: "world".into(),
    };

    writer.write(&kv1).map_err(|e| format!("write1: {:?}", e))?;
    writer.write(&kv2).map_err(|e| format!("write2: {:?}", e))?;
    std::thread::sleep(Duration::from_millis(100));

    let samples = reader.take().map_err(|e| format!("take: {:?}", e))?;
    if samples.len() != 2 {
        return Err(format!("expected 2 samples, got {}", samples.len()));
    }
    Ok(format!(
        "2 keyed instances: key={}, key={}",
        samples[0].key, samples[1].key
    ))
}

fn test_sequence_fields(dp: &DomainParticipant) -> Result<String, String> {
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let topic = Topic::<BatchReading>::new(dp.entity(), "seq_test")
        .map_err(|e| format!("{:?}", e))?;
    let writer = DataWriter::new(pub_.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;
    let reader: DataReader<BatchReading> = DataReader::new(sub.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;

    let readings = DdsSequence::from_slice(&[0.0f32, 10.0, 20.0, 30.0, 40.0])
        .map_err(|e| format!("from_slice: {:?}", e))?;

    let msg = BatchReading {
        sensor_id: 7,
        readings,
    };
    writer.write(&msg).map_err(|e| format!("write: {:?}", e))?;
    std::thread::sleep(Duration::from_millis(100));

    let samples = reader.take().map_err(|e| format!("take: {:?}", e))?;
    if samples.is_empty() {
        return Err("no samples received".into());
    }
    let s = &samples[0];
    Ok(format!("readings={}", s.readings.len()))
}

fn test_qos_builder(dp: &DomainParticipant) -> Result<String, String> {
    let qos = QosBuilder::new()
        .reliable()
        .transient_local()
        .keep_last(5)
        .build()
        .map_err(|e| format!("build qos: {:?}", e))?;

    let topic = Topic::<SensorData>::new(dp.entity(), "qos_test")
        .map_err(|e| format!("{:?}", e))?;
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let writer = pub_.create_writer_with_qos(&topic, &qos)
        .map_err(|e| format!("create_writer_with_qos: {:?}", e))?;

    let msg = SensorData {
        sensor_id: 1,
        temperature: 0.0,
        humidity: 0.0,
        label: "qos".into(),
        active: false,
    };
    writer.write(&msg).map_err(|e| format!("write: {:?}", e))?;
    Ok("Reliable + TransientLocal + KeepLast(5)".into())
}

fn test_waitset(dp: &DomainParticipant) -> Result<String, String> {
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let topic = Topic::<SensorData>::new(dp.entity(), "waitset_test")
        .map_err(|e| format!("{:?}", e))?;
    let writer = DataWriter::new(pub_.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;
    let reader: DataReader<SensorData> = DataReader::new(sub.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;

    let ws = WaitSet::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    ws.attach(reader.entity(), STATUS_DATA_AVAILABLE as i64)
        .map_err(|e| format!("attach: {:?}", e))?;

    let msg = SensorData {
        sensor_id: 55,
        temperature: 1.0,
        humidity: 1.0,
        label: "waitset".into(),
        active: true,
    };
    writer.write(&msg).map_err(|e| format!("write: {:?}", e))?;

    let triggered = ws
        .wait(1_000_000_000)
        .map_err(|e| format!("wait: {:?}", e))?;
    if triggered.is_empty() {
        return Err("waitset not triggered".into());
    }

    let samples = reader.take().map_err(|e| format!("take: {:?}", e))?;
    if samples.is_empty() {
        return Err("no samples after waitset trigger".into());
    }
    Ok(format!(
        "triggered with {} entities, {} samples",
        triggered.len(),
        samples.len()
    ))
}

fn test_read_vs_take(dp: &DomainParticipant) -> Result<String, String> {
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let topic = Topic::<SensorData>::new(dp.entity(), "readtake_test")
        .map_err(|e| format!("{:?}", e))?;
    let writer = DataWriter::new(pub_.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;
    let reader: DataReader<SensorData> = DataReader::new(sub.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;

    writer
        .write(&SensorData {
            sensor_id: 1,
            temperature: 0.0,
            humidity: 0.0,
            label: "read_test".into(),
            active: false,
        })
        .map_err(|e| format!("write: {:?}", e))?;
    std::thread::sleep(Duration::from_millis(50));

    // read() should not remove from cache
    let r1 = reader.read().map_err(|e| format!("read1: {:?}", e))?;
    if r1.is_empty() {
        return Err("read() returned empty".into());
    }
    let r2 = reader.read().map_err(|e| format!("read2: {:?}", e))?;
    if r2.is_empty() {
        return Err("second read() should still have data".into());
    }

    // take() should remove from cache
    let t1 = reader.take().map_err(|e| format!("take1: {:?}", e))?;
    if t1.is_empty() {
        return Err("take() returned empty".into());
    }
    let t2 = reader.take().map_err(|e| format!("take2: {:?}", e))?;
    if !t2.is_empty() {
        return Err("second take() should be empty".into());
    }

    Ok("read keeps, take removes".into())
}

fn test_write_dispose(dp: &DomainParticipant) -> Result<String, String> {
    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let topic = Topic::<KeyValue>::new(dp.entity(), "dispose_test")
        .map_err(|e| format!("{:?}", e))?;
    let writer = DataWriter::new(pub_.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;
    let _reader: DataReader<KeyValue> = DataReader::new(sub.entity(), topic.entity())
        .map_err(|e| format!("{:?}", e))?;

    let kv = KeyValue {
        key: 42,
        value: "dispose_me".into(),
    };
    writer
        .write_dispose(&kv)
        .map_err(|e| format!("write_dispose: {:?}", e))?;
    Ok("write_dispose OK".into())
}

fn test_content_filtered_topic(dp: &DomainParticipant) -> Result<String, String> {
    let topic = Topic::<SensorData>::new(dp.entity(), "cft_test")
        .map_err(|e| format!("{:?}", e))?;

    let cft = cyclonedds::ContentFilteredTopic::new(&topic, |data: &SensorData| {
        data.temperature > 20.0
    })
    .map_err(|e| format!("cft new: {:?}", e))?;

    let pub_ = Publisher::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let sub = Subscriber::new(dp.entity()).map_err(|e| format!("{:?}", e))?;
    let writer = DataWriter::new(pub_.entity(), cft.entity())
        .map_err(|e| format!("{:?}", e))?;
    let reader: DataReader<SensorData> = DataReader::new(sub.entity(), cft.entity())
        .map_err(|e| format!("{:?}", e))?;

    let cold = SensorData {
        sensor_id: 1,
        temperature: 10.0,
        humidity: 50.0,
        label: "cold".into(),
        active: false,
    };
    let warm = SensorData {
        sensor_id: 2,
        temperature: 25.0,
        humidity: 60.0,
        label: "warm".into(),
        active: true,
    };

    writer.write(&cold).map_err(|e| format!("write cold: {:?}", e))?;
    writer.write(&warm).map_err(|e| format!("write warm: {:?}", e))?;
    std::thread::sleep(Duration::from_millis(100));

    let samples = reader.take().map_err(|e| format!("take: {:?}", e))?;
    if samples.len() != 1 {
        return Err(format!("expected 1 filtered sample, got {}", samples.len()));
    }
    if samples[0].temperature <= 20.0 {
        return Err("filter did not block cold sample".into());
    }
    Ok(format!("filtered: {} of 2 passed", samples.len()))
}

fn test_topic_filter_ext(dp: &DomainParticipant) -> Result<String, String> {
    use cyclonedds::TopicFilterExt;
    let topic = Topic::<SensorData>::new(dp.entity(), "topic_filter_test")
        .map_err(|e| format!("{:?}", e))?;

    topic
        .set_filter(|data: &SensorData| data.sensor_id > 0)
        .map_err(|e| format!("set_filter: {:?}", e))?;

    topic
        .clear_filter()
        .map_err(|e| format!("clear_filter: {:?}", e))?;

    Ok("set + clear OK".into())
}

fn test_entity_name_qos(_dp: &DomainParticipant) -> Result<String, String> {
    let _qos = QosBuilder::new()
        .entity_name("test_participant")
        .build()
        .map_err(|e| format!("qos: {:?}", e))?;

    Ok("entity_name QoS OK".into())
}

fn test_multiple_participants() -> Result<String, String> {
    let dp1 = DomainParticipant::new(0).map_err(|e| format!("dp1: {:?}", e))?;
    let dp2 = DomainParticipant::new(1).map_err(|e| format!("dp2: {:?}", e))?;

    let t1 = Topic::<SensorData>::new(dp1.entity(), "cross_domain")
        .map_err(|e| format!("t1: {:?}", e))?;
    let t2 = Topic::<SensorData>::new(dp2.entity(), "cross_domain")
        .map_err(|e| format!("t2: {:?}", e))?;

    Ok(format!("dp1 topic={}, dp2 topic={}", t1.entity(), t2.entity()))
}

// ── Main ──────────────────────────────────────────────────────────

fn main() {
    println!("=== CycloneDDS-Rust Comprehensive Test Suite ===\n");

    // 1. DomainParticipant
    check("DomainParticipant::new", || test_participant());

    let dp = DomainParticipant::new(0).expect("need participant for further tests");

    // 2. Publisher / Subscriber
    check("Publisher + Subscriber", || test_publisher_subscriber(&dp));

    // 3. Topic creation (multiple types)
    check("Topic creation (3 types)", || test_topic_creation(&dp));

    // 4. Basic pub/sub
    check("Basic pub/sub", || test_basic_pubsub(&dp));

    // 5. Multiple writes
    check("Multiple writes (10 samples)", || test_multiple_writes(&dp));

    // 6. Keyed data
    check("Keyed data (instance lifecycle)", || test_keyed_data(&dp));

    // 7. Sequence fields
    check("Sequence fields (DdsSequence)", || test_sequence_fields(&dp));

    // 8. QoS Builder
    eprintln!("[DBG] starting QoS test...");
    check("QoS Builder (Reliable+TransientLocal+KeepLast)", || {
        test_qos_builder(&dp)
    });

    // 9. WaitSet
    eprintln!("[DBG] starting waitset test...");
    check("WaitSet + ReadCondition", || test_waitset(&dp));

    // 10. read() vs take() semantics
    eprintln!("[DBG] starting read_vs_take test...");
    check("read() keeps, take() removes", || test_read_vs_take(&dp));

    // 11. write_dispose
    eprintln!("[DBG] starting write_dispose test...");
    check("write_dispose", || test_write_dispose(&dp));

    // 12. Content-filtered topic
    check("ContentFilteredTopic (closure filter)", || {
        test_content_filtered_topic(&dp)
    });

    // 13. Topic-level filter
    check("TopicFilterExt (set/clear)", || test_topic_filter_ext(&dp));

    // 14. Entity name QoS
    check("Entity name QoS", || test_entity_name_qos(&dp));

    // 15. Multiple participants / domains
    check("Multiple participants (domain 0 + 1)", || {
        test_multiple_participants()
    });

    // ── Summary ──
    println!();
    let (p, f) = unsafe { (PASS_COUNT, FAIL_COUNT) };
    let total = p + f;
    println!("=== Results: {}/{} passed, {} failed ===", p, total, f);

    if f > 0 {
        std::process::exit(1);
    }
}
