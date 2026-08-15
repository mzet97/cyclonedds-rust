//! Dropping a sample stream must not leave a blocking wait running.
//!
//! `WaitSet::wait_async` runs `dds_waitset_wait` on `spawn_blocking`, and
//! `spawn_blocking` tasks cannot be cancelled: dropping the future that awaits
//! one detaches it, it does not stop it. So the question this file answers is
//! how long the blocking task keeps a runtime thread after the stream that
//! started it is gone.
//!
//! Measured through runtime shutdown, which is the only externally visible
//! consequence: `Runtime::drop` blocks until every blocking task has returned.
//! A stream is started with a timeout far longer than the test's patience, polled
//! once so the wait is genuinely in flight, then dropped.

use cyclonedds::{DataReader, DdsTypeDerive, DomainParticipant, Subscriber};
use cyclonedds_test_suite::unique_topic;
use futures_util::StreamExt;
use std::time::{Duration, Instant};

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct Idle {
    id: i32,
}

/// Longer than any patience this test has; if the wait runs to completion the
/// shutdown below takes this long.
const LONG_TIMEOUT_NS: i64 = 30_000_000_000;

#[test]
fn dropping_a_stream_mid_wait_does_not_hold_the_runtime() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic = participant
        .create_topic::<Idle>(&unique_topic("async_cancel_wait"))
        .unwrap();
    let subscriber = Subscriber::new(&participant).unwrap();
    let reader: DataReader<Idle> = DataReader::new(&subscriber, &topic).unwrap();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        let mut stream = Box::pin(reader.read_aiter_timeout(LONG_TIMEOUT_NS));
        // Poll once and give the blocking task time to reach dds_waitset_wait.
        // Nothing ever publishes on this topic, so this always times out.
        let _ = tokio::time::timeout(Duration::from_millis(300), stream.next()).await;
        // `stream` is dropped here, with the wait still in flight.
    });

    let start = Instant::now();
    drop(runtime);
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(5),
        "runtime shutdown blocked for {elapsed:?} waiting on the detached \
         dds_waitset_wait; it should be woken when the stream is dropped"
    );
}
