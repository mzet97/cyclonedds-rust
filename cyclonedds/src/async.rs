use crate::{DataReader, DdsEntity, DdsResult, DdsType, WaitSet};
use cyclonedds_sys::*;

#[cfg(feature = "async")]
impl WaitSet {
    pub async fn wait_async(&self, timeout_ns: i64) -> DdsResult<Vec<i64>> {
        let entity = self.entity();
        tokio::task::spawn_blocking(move || {
            let max_results: usize = 64;
            let mut xs: Vec<dds_attach_t> = vec![0; max_results];
            let n = unsafe { dds_waitset_wait(entity, xs.as_mut_ptr(), max_results, timeout_ns) };
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
    pub async fn take_async(&self) -> DdsResult<Vec<T>> {
        let entity = self.entity();
        tokio::task::spawn_blocking(move || unsafe {
            let max_samples: usize = 256;
            let mut samples: Vec<*mut std::ffi::c_void> = vec![std::ptr::null_mut(); max_samples];
            let mut infos: Vec<dds_sample_info> = vec![std::mem::zeroed(); max_samples];

            let n = dds_take(
                entity,
                samples.as_mut_ptr(),
                infos.as_mut_ptr() as *mut dds_sample_info_t,
                max_samples,
                max_samples as u32,
            );

            if n < 0 {
                return Err(crate::DdsError::from(n));
            }
            let n = n as usize;

            let mut result = Vec::with_capacity(n);
            for i in 0..n {
                if infos[i].valid_data && !samples[i].is_null() {
                    let data = std::ptr::read(samples[i] as *const T);
                    result.push(data);
                }
            }

            let _ = dds_return_loan(entity, samples.as_mut_ptr(), n as i32);
            Ok(result)
        })
        .await
        .map_err(|e| crate::DdsError::Other(e.to_string()))?
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
    pub fn read_aiter(&self) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        let entity = self.entity();
        async_stream::try_stream! {
            let participant = tokio::task::spawn_blocking(move || unsafe {
                dds_get_participant(entity)
            }).await.map_err(|e| crate::DdsError::Other(e.to_string()))?;

            let waitset = WaitSet::new(participant)?;
            waitset.attach(entity, 0)?;

            loop {
                let triggered = waitset.wait_async(dds_duration_t::MAX).await?;
                if triggered.is_empty() {
                    // timeout with no data — yield empty batch so caller
                    // can still make progress / apply back-pressure.
                    yield Vec::new();
                    continue;
                }

                let batch = tokio::task::spawn_blocking(move || unsafe {
                    let max_samples: usize = 256;
                    let mut samples: Vec<*mut std::ffi::c_void> = vec![std::ptr::null_mut(); max_samples];
                    let mut infos: Vec<dds_sample_info> = vec![std::mem::zeroed(); max_samples];

                    let n = dds_read(
                        entity,
                        samples.as_mut_ptr(),
                        infos.as_mut_ptr() as *mut dds_sample_info_t,
                        max_samples,
                        max_samples as u32,
                    );

                    if n < 0 {
                        return Err::<Vec<T>, crate::DdsError>(crate::DdsError::from(n));
                    }
                    let n = n as usize;

                    let mut result = Vec::with_capacity(n);
                    for i in 0..n {
                        if infos[i].valid_data && !samples[i].is_null() {
                            let data = std::ptr::read(samples[i] as *const T);
                            result.push(data);
                        }
                    }

                    let _ = dds_return_loan(entity, samples.as_mut_ptr(), n as i32);
                    Ok(result)
                })
                .await
                .map_err(|e| crate::DdsError::Other(e.to_string()))?;

                yield batch?;
            }
        }
    }

    /// Async iterator that yields batches of samples via `take`.
    ///
    /// Like [`read_aiter`](Self::read_aiter) but removes samples from the
    /// reader history cache.
    pub fn take_aiter(&self) -> impl futures_core::Stream<Item = DdsResult<Vec<T>>> + '_ {
        let entity = self.entity();
        async_stream::try_stream! {
            let participant = tokio::task::spawn_blocking(move || unsafe {
                dds_get_participant(entity)
            }).await.map_err(|e| crate::DdsError::Other(e.to_string()))?;

            let waitset = WaitSet::new(participant)?;
            waitset.attach(entity, 0)?;

            loop {
                let triggered = waitset.wait_async(dds_duration_t::MAX).await?;
                if triggered.is_empty() {
                    yield Vec::new();
                    continue;
                }

                let batch = tokio::task::spawn_blocking(move || unsafe {
                    let max_samples: usize = 256;
                    let mut samples: Vec<*mut std::ffi::c_void> = vec![std::ptr::null_mut(); max_samples];
                    let mut infos: Vec<dds_sample_info> = vec![std::mem::zeroed(); max_samples];

                    let n = dds_take(
                        entity,
                        samples.as_mut_ptr(),
                        infos.as_mut_ptr() as *mut dds_sample_info_t,
                        max_samples,
                        max_samples as u32,
                    );

                    if n < 0 {
                        return Err::<Vec<T>, crate::DdsError>(crate::DdsError::from(n));
                    }
                    let n = n as usize;

                    let mut result = Vec::with_capacity(n);
                    for i in 0..n {
                        if infos[i].valid_data && !samples[i].is_null() {
                            let data = std::ptr::read(samples[i] as *const T);
                            result.push(data);
                        }
                    }

                    let _ = dds_return_loan(entity, samples.as_mut_ptr(), n as i32);
                    Ok(result)
                })
                .await
                .map_err(|e| crate::DdsError::Other(e.to_string()))?;

                yield batch?;
            }
        }
    }
}
