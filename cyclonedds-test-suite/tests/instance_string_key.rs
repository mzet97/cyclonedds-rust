//! Regression coverage for instance operations on types whose key has a
//! heap-allocated field.
//!
//! `instances.rs` only ever exercised `KeyedMessage { key: i32, value: i32 }`,
//! a POD type where `T::Native` is layout-identical to `T`. Two reader-side
//! defects survived because of that:
//!
//! * `DataReader::instance_get_key` built the output with
//!   `let mut data: T = std::mem::zeroed()` — an all-zero `String` violates the
//!   `NonNull` niche inside its `Vec`, the buffer is sized `size_of::<T>()`
//!   while CycloneDDS writes the `Native` layout, and the returned value is
//!   then freed by Rust over memory allocated by `ddsrt_malloc`.
//! * `DataReader::lookup_instance` passed `&T` straight to
//!   `dds_lookup_instance`, while every equivalent writer-side method routes
//!   through `write_to_native`. CycloneDDS reads the key at the *native*
//!   offset and finds the middle of Rust's `String`.
//!
//! Run under AddressSanitizer to catch the corruption deterministically:
//! ```bash
//! RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p cyclonedds-test-suite \
//!     --target x86_64-unknown-linux-gnu --test instance_string_key
//! ```

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct StringKeyed {
    #[key]
    name: String,
    value: i32,
}

#[test]
fn lookup_instance_agrees_between_reader_and_writer() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<StringKeyed>(&unique_topic("string_key_lookup"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let alpha = StringKeyed {
        name: "alpha".to_string(),
        value: 1,
    };
    let beta = StringKeyed {
        name: "beta".to_string(),
        value: 2,
    };

    let alpha_handle = writer.register_instance(&alpha).unwrap();
    let beta_handle = writer.register_instance(&beta).unwrap();
    assert_ne!(alpha_handle, 0);
    assert_ne!(beta_handle, 0);
    assert_ne!(alpha_handle, beta_handle);

    writer.write(&alpha).unwrap();
    writer.write(&beta).unwrap();

    assert!(wait_for(Duration::from_secs(2), || reader
        .read()
        .unwrap_or_default()
        .len()
        >= 2));

    // The writer path already routes through `write_to_native` and is correct.
    assert_eq!(writer.lookup_instance(&alpha), alpha_handle);

    // The reader must resolve the same instance for the same key. With the
    // raw-`&T` path CycloneDDS hashes the wrong bytes and returns 0 or a
    // handle belonging to a different instance.
    let from_reader = reader.lookup_instance(&alpha);
    assert_ne!(
        from_reader, 0,
        "reader.lookup_instance failed to resolve a String key"
    );
    assert_eq!(
        from_reader, alpha_handle,
        "reader and writer disagree on the instance handle for the same key"
    );
    assert_ne!(reader.lookup_instance(&beta), from_reader);
}

#[test]
fn instance_get_key_round_trips_a_string_key() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<StringKeyed>(&unique_topic("string_key_get"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    short_delay();

    let sample = StringKeyed {
        name: "gamma".to_string(),
        value: 33,
    };
    // Instance handles are derived from the key hash and are shared across
    // endpoints, so the writer-side handle is the one to query on the reader —
    // same pattern as `instances.rs`.
    let handle = writer.register_instance(&sample).unwrap();
    assert_ne!(handle, 0);
    writer.write(&sample).unwrap();

    assert!(wait_for(Duration::from_secs(2), || !reader
        .read()
        .unwrap_or_default()
        .is_empty()));
    assert!(!reader.take().unwrap().is_empty());

    let recovered = reader
        .instance_get_key(handle)
        .expect("instance_get_key failed");
    assert_eq!(
        recovered.name, "gamma",
        "key field did not survive instance_get_key"
    );

    // Dropping `recovered` must not free CycloneDDS-owned memory.
    drop(recovered);
}
