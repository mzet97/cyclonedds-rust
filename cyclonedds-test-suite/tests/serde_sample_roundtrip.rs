//! `SerdeSample<T>` must survive an actual DDS round-trip.
//!
//! The `serde` feature had no test coverage at all, which is how this shipped:
//! `SerdeSample` declared `Native = Self` and handed the Rust struct straight to
//! CycloneDDS. Its ops say `ADR | SEQ | 1BY`, so the C side reads that memory as
//! `dds_sequence_t` (`{u32 _maximum, u32 _length, *mut u8 _buffer, bool
//! _release}`) — but it held a `Vec<u8>`, whose field order is `repr(Rust)` and
//! explicitly not guaranteed. `clone_out` then did `ptr::read`, taking Rust
//! ownership of a buffer `dds_alloc` had allocated.
//!
//! The same module was already missed once, when `DdsType::Native` was
//! introduced in 2.0.0 and this `impl` was overlooked — see the 2.0.1 entry.
//! Nobody builds with `--features serde`, so nothing caught either.
//!
//! Run this suite explicitly; it is behind the feature:
//! ```bash
//! cargo test -p cyclonedds-test-suite --features serde --test serde_sample_roundtrip
//! ```
#![cfg(feature = "serde")]

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Telemetry {
    id: u32,
    label: String,
    readings: Vec<f64>,
}

fn sample() -> Telemetry {
    Telemetry {
        id: 4242,
        label: "sensor-alpha".to_string(),
        readings: vec![1.5, -2.25, 3.125],
    }
}

#[test]
fn serde_sample_round_trips_over_dds() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<SerdeSample<Telemetry>>(&unique_topic("serde_rt"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let original = sample();
    writer
        .write(&SerdeSample::new(&original).unwrap())
        .unwrap();

    assert!(
        wait_for(Duration::from_secs(3), || !reader
            .read()
            .unwrap_or_default()
            .is_empty()),
        "sample never arrived"
    );

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty());

    // The payload must survive the trip byte for byte, and still deserialize.
    let recovered: Telemetry = taken[0].deserialize().expect("deserialize failed");
    assert_eq!(recovered, original);
}

/// Repeat enough that a per-sample double free or allocator mismatch shows up.
#[test]
fn repeated_round_trips_do_not_corrupt() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<SerdeSample<Telemetry>>(&unique_topic("serde_rt_many"))
        .unwrap();
    // KEEP_ALL + reliable, otherwise the default KEEP_LAST(1) retains a single
    // sample and the loop below proves nothing about repeated allocations.
    let qos = Qos::builder().reliable().keep_all().build().unwrap();
    let writer = publisher.create_writer_with_qos(&topic, &qos).unwrap();
    let reader = subscriber.create_reader_with_qos(&topic, &qos).unwrap();

    short_delay();

    for i in 0..32u32 {
        let mut s = sample();
        s.id = i;
        writer.write(&SerdeSample::new(&s).unwrap()).unwrap();
    }

    assert!(wait_for(Duration::from_secs(3), || reader
        .read()
        .unwrap_or_default()
        .len()
        >= 8));

    let taken = reader.take().unwrap();
    assert!(!taken.is_empty(), "no samples taken");
    for s in &taken {
        let t: Telemetry = s.deserialize().expect("deserialize failed");
        assert_eq!(t.label, "sensor-alpha");
        assert_eq!(t.readings, vec![1.5, -2.25, 3.125]);
    }
}

/// `as_bytes` must give back exactly what postcard produced, unchanged by the
/// trip through the DDS sequence.
#[test]
fn payload_bytes_are_unchanged() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<SerdeSample<Telemetry>>(&unique_topic("serde_rt_bytes"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let original = SerdeSample::new(&sample()).unwrap();
    let expected = original.as_bytes().to_vec();
    assert!(!expected.is_empty());

    writer.write(&original).unwrap();
    assert!(wait_for(Duration::from_secs(3), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));

    let taken = reader.take().unwrap();
    assert_eq!(taken[0].as_bytes(), expected.as_slice());
}
