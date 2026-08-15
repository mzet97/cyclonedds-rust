use crate::entity::OwnedHandle;
use crate::{DataReader, DdsEntity, DdsResult, DdsType, WaitSet};
use cyclonedds_rust_sys::*;

/// Default batch size for the stream methods that do not take one explicitly.
const DEFAULT_BATCH: usize = 256;

/// Cookie the reader is attached to a stream's waitset under.
const READER_COOKIE: i64 = 0;
/// Cookie the waitset is attached to *itself* under, to wake its own wait.
const INTERRUPT_COOKIE: i64 = 1;

/// A stream's waitset, which wakes any in-flight wait when the stream is dropped.
///
/// The blocking task holds an `Arc` of the waitset (see
/// [`WaitSet::wait_async`]), which is what keeps the wait off a recycled handle
/// — but it also means the waitset is no longer deleted when the stream is
/// dropped, and deletion was what used to interrupt the wait. Without this guard
/// the blocking task then sits for the full timeout: measured at 29.7s for a
/// 30s-timeout stream in `tests/async_wait_cancellation.rs`.
///
/// `Drop::drop` runs before the field is dropped, so the sequence is: trigger,
/// the blocking task's `dds_waitset_wait` returns and releases its `Arc`, then
/// the waitset is deleted once the last reference goes.
struct StreamWaitSet(WaitSet);

impl Drop for StreamWaitSet {
    fn drop(&mut self) {
        // Nothing to do if it fails: the waitset is already gone, which wakes
        // the wait by itself.
        let _ = self.0.set_trigger(true);
    }
}

/// Drain up to `max_samples` from a reader's history cache.
///
/// `dds_read` / `dds_take` do **not** block: they walk the reader history cache
/// and return immediately. This therefore runs inline on the caller's thread.
///
/// It used to be wrapped in `tokio::task::spawn_blocking`, which bought nothing
/// — the call never blocks — and opened a soundness hole. `spawn_blocking`
/// requires a `'static` task, so only the raw `dds_entity_t` (an `i32`, `Copy`)
/// could be moved in, not a borrow of the reader. Cancelling the future
/// (`tokio::select!`, a timeout, dropping the stream) left that task running
/// against a handle whose `DataReader` may already have been dropped and its
/// entity deleted — and CycloneDDS recycles entity handles, so the call could
/// land on an unrelated entity created in the meantime. Running inline ties the
/// call to the borrow of `&self` that the future already holds.
///
/// # Safety
/// `entity` must be a live reader entity for type `T`.
unsafe fn drain<T: DdsType>(
    entity: dds_entity_t,
    take: bool,
    max_samples: usize,
) -> DdsResult<Vec<T>> {
    let mut samples: Vec<*mut std::ffi::c_void> = vec![std::ptr::null_mut(); max_samples];
    let mut infos: Vec<dds_sample_info> = vec![std::mem::zeroed(); max_samples];

    let n = if take {
        dds_take(
            entity,
            samples.as_mut_ptr(),
            infos.as_mut_ptr() as *mut dds_sample_info_t,
            max_samples,
            max_samples as u32,
        )
    } else {
        dds_read(
            entity,
            samples.as_mut_ptr(),
            infos.as_mut_ptr() as *mut dds_sample_info_t,
            max_samples,
            max_samples as u32,
        )
    };

    if n < 0 {
        return Err(crate::DdsError::from(n));
    }
    let n = n as usize;

    let mut result = Vec::with_capacity(n);
    for i in 0..n {
        if infos[i].valid_data && !samples[i].is_null() {
            // clone_out converte a amostra nativa (DdsString/sequências) em valor
            // Rust próprio — ptr::read aqui seria UB: a amostra nativa tem layout
            // C (strings = char* de 8B, não String de 24B) e o buffer é devolvido
            // ao DDS no return_loan abaixo.
            // Amostra indecodificável é pulada, não derruba o lote inteiro.
            if let Ok(data) = T::clone_out(samples[i] as *const T) {
                result.push(data);
            }
        }
    }

    let _ = dds_return_loan(entity, samples.as_mut_ptr(), n as i32);
    Ok(result)
}

#[cfg(feature = "async")]
impl WaitSet {
    /// Wait for any attached entity to trigger, off the async runtime.
    ///
    /// Unlike `dds_read`/`dds_take`, `dds_waitset_wait` genuinely blocks, so it
    /// stays on `spawn_blocking`.
    ///
    /// # Cancellation
    ///
    /// `spawn_blocking` tasks are not cancellable: dropping the future that
    /// awaits this detaches the blocking task, it does not stop it. That is
    /// bounded rather than open-ended, though, and the reason is on the C side.
    /// Deleting a waitset runs `dds_waitset_interrupt`
    /// (`vendor/cyclonedds/.../dds_waitset.c:92`, wired in at `:137`), which
    /// broadcasts the wait condition, and the wait loop rechecks
    /// `dds_handle_is_closed` — so an in-flight `dds_waitset_wait` returns as
    /// soon as its waitset goes away, without waiting out the timeout.
    /// `tests/async_wait_cancellation.rs` measures this through runtime
    /// shutdown: dropping a stream that is mid-wait on a 30-second timeout
    /// releases the runtime thread in well under a second.
    ///
    /// What that leaves is a handle race, not a hang. The task used to capture
    /// only the raw `dds_entity_t`, so between the waitset's `dds_delete` and
    /// the task's `dds_entity_pin` the handle could in principle be redrawn for
    /// a different entity — the same window measured for A1: `dds_handle_create`
    /// draws from ~2.1e9 values, so this is rare rather than impossible, and it
    /// is argued, not demonstrated. The task now holds an `Arc` of the waitset,
    /// which closes it by construction: the entity cannot be deleted while the
    /// wait is running.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub async fn wait_async(&self, timeout_ns: i64) -> DdsResult<Vec<i64>> {
        // Cloned, not borrowed: the task can outlive this future.
        let owned = self.owned().clone();
        tokio::task::spawn_blocking(move || {
            let max_results: usize = 64;
            let mut xs: Vec<dds_attach_t> = vec![0; max_results];
            let n = unsafe {
                dds_waitset_wait(owned.handle(), xs.as_mut_ptr(), max_results, timeout_ns)
            };
            // Keep the waitset alive across the call, not merely up to it.
            drop(owned);
            if n < 0 {
                return Err(crate::DdsError::from(n));
            }
            let n = n as usize;
            xs.truncate(n);
            Ok(xs.into_iter().map(|x| x as i64).collect())
        })
        .await
        .map_err(|e| crate::DdsError::Other(e.to_string()))?
    }
}

#[cfg(feature = "async")]
impl<T: DdsType> DataReader<T> {
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub async fn take_async(&self) -> DdsResult<Vec<T>> {
        // `dds_take` does not block — see `drain`.
        unsafe { drain::<T>(self.entity(), true, DEFAULT_BATCH) }
    }

    /// Shared implementation behind every `*_aiter*` method.
    ///
    /// Waits on a `WaitSet` for data, then drains the history cache inline.
    fn sample_stream(
        &self,
        take: bool,
        max_samples: usize,
        timeout_ns: i64,
    ) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        let entity = self.entity();
        // The waitset holds the reader (and through it the subscriber, topic
        // and participant) alive for as long as the stream does, so a wait can
        // never be left sitting on an entity someone else deleted.
        let owner = self.owned().clone();
        async_stream::try_stream! {
            // Non-blocking lookup; no reason to hop threads for it.
            let participant = unsafe { dds_get_participant(entity) };

            let waitset = WaitSet::for_parents(participant, vec![owner])?;
            waitset.attach(entity, READER_COOKIE)?;
            // Observe ourselves, so `StreamWaitSet`'s drop can wake a wait that
            // is still in flight. This is the mechanism CycloneDDS documents for
            // it: a waitset is "triggered when trigger value was set to true by
            // the application ... can be used to wake up a waitset for different
            // reasons (f.i. termination)".
            waitset.attach(waitset.entity(), INTERRUPT_COOKIE)?;
            let waitset = StreamWaitSet(waitset);

            loop {
                let triggered = waitset.0.wait_async(timeout_ns).await?;
                if triggered.contains(&INTERRUPT_COOKIE) {
                    // Only the drop guard sets this, and the guard runs when the
                    // generator is already being torn down; returning is tidiness,
                    // not control flow anyone observes.
                    break;
                }
                if triggered.is_empty() {
                    // Timeout with no data — yield an empty batch so the caller
                    // can still make progress / apply back-pressure.
                    yield Vec::new();
                    continue;
                }

                yield unsafe { drain::<T>(entity, take, max_samples) }?;
            }
        }
    }

    /// Async iterator that yields batches of samples via `read`.
    ///
    /// The stream waits for new data using a [`WaitSet`] and then reads
    /// all available samples.  It yields `Vec<T>` (possibly empty on timeout)
    /// and continues until the stream is dropped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cyclonedds::DataReader;
    /// use futures_util::StreamExt;
    /// # async fn example<T: cyclonedds::DdsType>(reader: &DataReader<T>) {
    /// let mut stream = Box::pin(reader.read_aiter());
    /// while let Some(batch) = stream.next().await {
    ///     match batch {
    ///         Ok(samples) => println!("got {} samples", samples.len()),
    ///         Err(e) => eprintln!("read error: {}", e),
    ///     }
    /// }
    /// # }
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn read_aiter(&self) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        self.sample_stream(false, DEFAULT_BATCH, dds_duration_t::MAX)
    }

    /// Async iterator that yields batches of samples via `read`, with a
    /// configurable maximum number of samples per batch.
    ///
    /// This is useful when you expect large bursts of data and want to
    /// process them in fixed-size chunks to apply back-pressure.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cyclonedds::DataReader;
    /// use futures_util::StreamExt;
    /// # async fn example<T: cyclonedds::DdsType>(reader: &DataReader<T>) {
    /// let mut stream = Box::pin(reader.read_aiter_batch(64));
    /// while let Some(batch) = stream.next().await {
    ///     match batch {
    ///         Ok(samples) => println!("got {} samples", samples.len()),
    ///         Err(e) => eprintln!("read error: {}", e),
    ///     }
    /// }
    /// # }
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn read_aiter_batch(
        &self,
        max_samples: usize,
    ) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        self.sample_stream(false, max_samples, dds_duration_t::MAX)
    }

    /// Async iterator that yields batches of samples via `take`.
    ///
    /// Like [`read_aiter`](Self::read_aiter) but removes samples from the
    /// reader history cache.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self)))]
    pub fn take_aiter(&self) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        self.sample_stream(true, DEFAULT_BATCH, dds_duration_t::MAX)
    }

    /// Async iterator that yields batches of samples via `take`, with a
    /// configurable maximum number of samples per batch.
    ///
    /// Like [`take_aiter`](Self::take_aiter) but allows limiting the batch
    /// size for back-pressure control.
    pub fn take_aiter_batch(
        &self,
        max_samples: usize,
    ) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        self.sample_stream(true, max_samples, dds_duration_t::MAX)
    }

    /// Async iterator that yields batches of samples via `read` with a
    /// configurable timeout on the WaitSet.
    ///
    /// If no data arrives within `timeout_ns`, the stream yields an empty
    /// `Vec` and continues.  This makes the stream compatible with
    /// `tokio::select!` and other cancellation mechanisms.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cyclonedds::DataReader;
    /// use futures_util::StreamExt;
    /// # async fn example<T: cyclonedds::DdsType>(reader: &DataReader<T>) {
    /// let mut stream = Box::pin(reader.read_aiter_timeout(1_000_000_000));
    /// while let Some(batch) = stream.next().await {
    ///     match batch {
    ///         Ok(samples) if !samples.is_empty() => println!("got {} samples", samples.len()),
    ///         Ok(_) => println!("timeout — no data"),
    ///         Err(e) => eprintln!("read error: {}", e),
    ///     }
    /// }
    /// # }
    /// ```
    pub fn read_aiter_timeout(
        &self,
        timeout_ns: i64,
    ) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        self.sample_stream(false, DEFAULT_BATCH, timeout_ns)
    }

    /// Async iterator that yields batches of samples via `read` with both
    /// a configurable batch size and a timeout on the WaitSet.
    ///
    /// This combines [`read_aiter_batch`](Self::read_aiter_batch) and
    /// [`read_aiter_timeout`](Self::read_aiter_timeout) for fine-grained
    /// back-pressure and cancellation control.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cyclonedds::DataReader;
    /// use futures_util::StreamExt;
    /// # async fn example<T: cyclonedds::DdsType>(reader: &DataReader<T>) {
    /// let mut stream = Box::pin(reader.read_aiter_batch_timeout(64, 500_000_000));
    /// while let Some(batch) = stream.next().await {
    ///     match batch {
    ///         Ok(samples) if !samples.is_empty() => println!("got {} samples", samples.len()),
    ///         Ok(_) => println!("timeout — no data"),
    ///         Err(e) => eprintln!("read error: {}", e),
    ///     }
    /// }
    /// # }
    /// ```
    pub fn read_aiter_batch_timeout(
        &self,
        max_samples: usize,
        timeout_ns: i64,
    ) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        self.sample_stream(false, max_samples, timeout_ns)
    }

    /// Async iterator that yields batches of samples via `take` with both
    /// a configurable batch size and a timeout on the WaitSet.
    ///
    /// Like [`read_aiter_batch_timeout`](Self::read_aiter_batch_timeout) but
    /// removes samples from the reader history.
    pub fn take_aiter_batch_timeout(
        &self,
        max_samples: usize,
        timeout_ns: i64,
    ) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        self.sample_stream(true, max_samples, timeout_ns)
    }

    /// Async iterator that yields batches of samples via `take` with a
    /// configurable timeout on the WaitSet.
    ///
    /// Like [`read_aiter_timeout`](Self::read_aiter_timeout) but removes
    /// samples from the reader history.
    pub fn take_aiter_timeout(
        &self,
        timeout_ns: i64,
    ) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        self.sample_stream(true, DEFAULT_BATCH, timeout_ns)
    }
}
