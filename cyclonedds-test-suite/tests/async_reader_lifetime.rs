//! The async read path must not outlive the reader it borrows.
//!
//! `take_async` (and the drain step of every `*_aiter*` stream) used to run its
//! `dds_take` inside `tokio::task::spawn_blocking`. That task is `'static`, so
//! only the raw `dds_entity_t` — an `i32` — was moved into it, not a borrow of
//! the `DataReader`. Cancelling the future left the task running against a
//! handle whose reader could already have been dropped and its entity deleted,
//! and CycloneDDS recycles entity handles: the call could land on an unrelated
//! entity created in the meantime.
//!
//! `dds_take`/`dds_read` never block (they walk the reader history cache), so
//! the thread hop bought nothing. They now run inline, tied to the `&self`
//! borrow the future already holds.
//!
//! Run under AddressSanitizer for a deterministic verdict:
//! ```bash
//! RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p cyclonedds-test-suite \
//!     --target x86_64-unknown-linux-gnu --test async_reader_lifetime
//! ```

use cyclonedds::*;
use cyclonedds_test_suite::{short_delay, unique_topic};
use futures_util::StreamExt;
use std::time::Duration;

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct AsyncMessage {
    #[key]
    id: i32,
    text: String,
}

/// Cancel `take_async` and drop the reader immediately, many times over. With
/// the old `spawn_blocking` path this raced a detached task against a deleted
/// (and possibly recycled) entity handle.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancelled_take_async_does_not_outlive_the_reader() {
    let participant = DomainParticipant::new(0).unwrap();
    let subscriber = participant.create_subscriber().unwrap();

    for i in 0..32 {
        let topic = participant
            .create_topic::<AsyncMessage>(&unique_topic(&format!("async_life_{i}")))
            .unwrap();
        let reader =
            DataReader::<AsyncMessage>::new(subscriber.entity(), topic.entity()).unwrap();

        // Cancel the read almost immediately.
        let outcome = tokio::time::timeout(Duration::from_micros(1), reader.take_async()).await;
        match outcome {
            Ok(Ok(samples)) => assert!(samples.is_empty()),
            Ok(Err(e)) => panic!("take_async failed: {e}"),
            Err(_elapsed) => {} // cancelled — the case under test
        }

        // Drop the reader (and its entity) right after cancelling.
        drop(reader);
        drop(topic);
    }
}

/// Same shape for the stream path: drop a live stream, then the reader.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dropped_stream_does_not_outlive_the_reader() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();

    for i in 0..8 {
        let topic = participant
            .create_topic::<AsyncMessage>(&unique_topic(&format!("async_stream_life_{i}")))
            .unwrap();
        let writer = publisher.create_writer(&topic).unwrap();
        let reader =
            DataReader::<AsyncMessage>::new(subscriber.entity(), topic.entity()).unwrap();

        short_delay();
        writer
            .write(&AsyncMessage {
                id: i,
                text: format!("msg-{i}"),
            })
            .unwrap();

        {
            let mut stream = Box::pin(reader.take_aiter_timeout(50_000_000));
            let _ = stream.next().await;
            // stream dropped here, mid-life
        }

        drop(reader);
        drop(writer);
        drop(topic);
    }
}

/// The refactor must not change observable behaviour: data still flows.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn take_async_still_delivers_samples() {
    let participant = DomainParticipant::new(0).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let topic = participant
        .create_topic::<AsyncMessage>(&unique_topic("async_delivers"))
        .unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = DataReader::<AsyncMessage>::new(subscriber.entity(), topic.entity()).unwrap();

    short_delay();
    writer
        .write(&AsyncMessage {
            id: 7,
            text: "async-payload".to_string(),
        })
        .unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        let batch = reader.take_async().await.unwrap();
        if let Some(first) = batch.first() {
            assert_eq!(first.id, 7);
            assert_eq!(first.text, "async-payload");
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "sample never arrived through take_async"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}
