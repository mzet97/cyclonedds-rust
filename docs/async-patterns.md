# Async Patterns

This guide covers common asynchronous patterns when using `cyclonedds` with tokio.

## Timeouts on Streams

All async iterators support built-in timeouts via the `timeout_ns` parameter:

```rust
use cyclonedds::DataReader;
use futures_util::StreamExt;

async fn example<T: cyclonedds::DdsType>(reader: &DataReader<T>) {
    // Yields empty Vec after 1 second if no data arrives
    let mut stream = Box::pin(reader.read_aiter_timeout(1_000_000_000));
    while let Some(batch) = stream.next().await {
        match batch {
            Ok(samples) if !samples.is_empty() => println!("got {} samples", samples.len()),
            Ok(_) => println!("timeout — no data"),
            Err(e) => eprintln!("read error: {}", e),
        }
    }
}
```

Available timeout variants:
- `read_aiter_timeout(timeout_ns)`
- `read_aiter_batch_timeout(max_samples, timeout_ns)`
- `take_aiter_timeout(timeout_ns)`
- `take_aiter_batch_timeout(max_samples, timeout_ns)`

## Cancellation with `tokio::select!`

Because timeout variants yield empty batches instead of blocking forever,
they are safe to use inside `tokio::select!`:

```rust
use cyclonedds::DataReader;
use futures_util::StreamExt;
use tokio::time::{sleep, Duration};

async fn example<T: cyclonedds::DdsType>(reader: &DataReader<T>) {
    let mut stream = Box::pin(reader.read_aiter_timeout(500_000_000));

    loop {
        tokio::select! {
            Some(batch) = stream.next() => {
                match batch {
                    Ok(samples) if !samples.is_empty() => {
                        println!("received {} samples", samples.len());
                    }
                    Ok(_) => { /* timeout — continue */ }
                    Err(e) => eprintln!("error: {}", e),
                }
            }
            _ = sleep(Duration::from_secs(5)) => {
                println!("shutting down after 5 seconds");
                break;
            }
        }
    }
}
```

Dropping the stream (e.g. when the `select!` branch is cancelled) cleans up
the underlying WaitSet and reader attachments automatically.

## Back-Pressure with Batch Limits

Use `*_batch_timeout` variants to limit the number of samples per iteration:

```rust
// Process at most 64 samples every 500ms
let mut stream = Box::pin(reader.take_aiter_batch_timeout(64, 500_000_000));
```

This prevents a fast publisher from overwhelming a slow consumer.

## Async Single Take

For one-shot async reads, use `take_async`:

```rust
let samples = reader.take_async().await?;
```

This runs the DDS take operation in a `spawn_blocking` task to avoid
blocking the async runtime.

## WaitSet Timeouts

The underlying `WaitSet::wait_async` also supports timeouts:

```rust
let triggered = waitset.wait_async(1_000_000_000).await?;
if triggered.is_empty() {
    println!("WaitSet timed out after 1 second");
}
```
