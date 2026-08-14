//! Regression coverage for the raw-CDR read path (`read_cdr` / `take_cdr`).
//!
//! This surface had **zero** test coverage, which is how the `ddsi_serdata`
//! vtable helpers in `cyclonedds-rust-sys` shipped reading a single byte where
//! an 8-byte function pointer was intended (`*(ops as *const u8).add(N)`
//! dereferences a `u8`, not a `*const fn`). Every call to `ddsi_serdata_size`
//! or `ddsi_serdata_to_ser` therefore transmuted a value in `0..=255` into a
//! function pointer and called it.
//!
//! Note: before the fix these tests do not *fail*, they **abort the test
//! binary** with a segfault, because the jump target is an unmapped page. Run
//! with `--test-threads=1` to see which one died.

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct CdrMessage {
    #[key]
    id: i32,
    value: i32,
}

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct CdrStringMessage {
    #[key]
    id: i32,
    text: String,
}

/// Exercises `ddsi_serdata_size` + `ddsi_serdata_to_ser` + `ddsi_serdata_unref`.
#[test]
fn read_cdr_returns_serialized_bytes() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<CdrMessage>(&unique_topic("cdr_read"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer.write(&CdrMessage { id: 7, value: 42 }).unwrap();

    assert!(
        wait_for(Duration::from_secs(2), || !reader
            .read()
            .unwrap_or_default()
            .is_empty()),
        "sample never arrived"
    );

    let samples = reader.read_cdr().unwrap();
    assert!(!samples.is_empty(), "read_cdr returned no samples");

    // A CDR payload always carries a 4-byte encapsulation header, so any
    // plausible serialization of this type is strictly larger than that.
    // A bogus `ddsi_serdata_size` would report a nonsense length here.
    assert!(
        samples[0].data.len() > 4,
        "implausible CDR length {} — ddsi_serdata_size returned garbage",
        samples[0].data.len()
    );
    assert!(samples[0].info.valid_data);
}

/// `take_cdr` removes from the reader cache; also covers the unref path more
/// aggressively since the samples are dropped rather than retained.
#[test]
fn take_cdr_consumes_samples() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<CdrMessage>(&unique_topic("cdr_take"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();
    writer.write(&CdrMessage { id: 1, value: 10 }).unwrap();
    writer.write(&CdrMessage { id: 2, value: 20 }).unwrap();

    assert!(wait_for(Duration::from_secs(2), || reader
        .read()
        .unwrap_or_default()
        .len()
        >= 2));

    let taken = reader.take_cdr().unwrap();
    assert!(taken.len() >= 2, "take_cdr returned {} samples", taken.len());
    for sample in &taken {
        assert!(sample.data.len() > 4);
    }

    // Consumed: a second take must not see them again.
    assert!(reader.take_cdr().unwrap().is_empty());
}

/// Repeated calls stress `ddsi_serdata_unref` until a refcount actually
/// reaches zero and the vtable `free` slot is invoked.
#[test]
fn repeated_take_cdr_does_not_corrupt() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<CdrStringMessage>(&unique_topic("cdr_repeat"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    for i in 0..16 {
        writer
            .write(&CdrStringMessage {
                id: i,
                text: format!("payload-{i}"),
            })
            .unwrap();
    }

    assert!(wait_for(Duration::from_secs(3), || reader
        .read()
        .unwrap_or_default()
        .len()
        >= 16));

    let mut total = 0usize;
    for _ in 0..8 {
        total += reader.take_cdr().unwrap().len();
    }
    assert!(total >= 16, "only observed {total} CDR samples");
}
