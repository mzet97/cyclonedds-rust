//! A panicking listener callback must not take the process down.
//!
//! CycloneDDS invokes listener callbacks from its own threads. A Rust panic
//! escaping an `extern "C"` frame aborts the process on Rust >= 1.81 (and was
//! UB before), so a single `unwrap()` in a user closure used to be enough to
//! kill the whole application. None of the 13 listener trampolines had a
//! `catch_unwind` barrier, even though `waitset.rs` documented the rule and
//! applied it.
//!
//! If the barrier regresses, these tests do not fail — the test binary dies
//! with an abort, which is itself the signal.

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic, wait_for};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct PanicMessage {
    #[key]
    id: i32,
    text: String,
}

#[test]
fn panicking_data_available_callback_is_contained() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<PanicMessage>(&unique_topic("listener_panic"))
        .unwrap();

    let fired = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&fired);

    let listener = Listener::builder()
        .on_data_available(move |_reader| {
            seen.fetch_add(1, Ordering::SeqCst);
            panic!("panic injetado dentro do callback do listener");
        })
        .build()
        .unwrap();

    let reader = DataReader::<PanicMessage>::with_listener(&subscriber, &topic, &listener).unwrap();
    let writer = publisher.create_writer(&topic).unwrap();

    short_delay();
    writer
        .write(&PanicMessage {
            id: 1,
            text: "boom".to_string(),
        })
        .unwrap();

    assert!(
        wait_for(Duration::from_secs(3), || fired.load(Ordering::SeqCst) > 0),
        "the listener callback never ran"
    );

    // Surviving to this point is the assertion: the panic was contained at the
    // FFI boundary instead of aborting. The reader must still be usable.
    let _ = reader.take().unwrap();
}

/// A panicking callback must not poison the listener for subsequent samples.
#[test]
fn listener_keeps_working_after_a_panicking_callback() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<PanicMessage>(&unique_topic("listener_panic_repeat"))
        .unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let seen = Arc::clone(&calls);

    let listener = Listener::builder()
        .on_data_available(move |_reader| {
            let n = seen.fetch_add(1, Ordering::SeqCst);
            if n % 2 == 0 {
                panic!("panic alternado");
            }
        })
        .build()
        .unwrap();

    let _reader =
        DataReader::<PanicMessage>::with_listener(&subscriber, &topic, &listener).unwrap();
    let writer = publisher.create_writer(&topic).unwrap();

    short_delay();
    for i in 0..6 {
        writer
            .write(&PanicMessage {
                id: i,
                text: format!("msg-{i}"),
            })
            .unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }

    assert!(
        wait_for(Duration::from_secs(3), || calls.load(Ordering::SeqCst) >= 2),
        "callback stopped being invoked after the first panic (got {})",
        calls.load(Ordering::SeqCst)
    );
}
