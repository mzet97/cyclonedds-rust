use crate::{
    entity::DdsEntity,
    error::{check, check_entity},
    serialization::CdrSerializer,
    write_arena::WriteArena,
    xtypes::MatchedEndpoint,
    DdsResult, DdsType, Listener, Qos,
};
use cyclonedds_rust_sys::*;
use std::ffi::c_void;
use std::marker::PhantomData;

/// A typed DDS DataWriter that publishes samples of type `T`.
pub struct DataWriter<T: DdsType> {
    entity: dds_entity_t,
    _marker: PhantomData<T>,
}

impl<T: DdsType> DataWriter<T> {
    fn with_native_ptr<R>(
        &self,
        data: &T,
        f: impl FnOnce(*const c_void) -> DdsResult<R>,
    ) -> DdsResult<R> {
        let mut arena = WriteArena::new();
        let ptr = data.write_to_native(&mut arena)?;
        f(ptr)
    }

    pub fn new(publisher: dds_entity_t, topic: dds_entity_t) -> DdsResult<Self> {
        Self::with_qos_and_listener(publisher, topic, None, None)
    }

    pub fn with_qos(
        publisher: dds_entity_t,
        topic: dds_entity_t,
        qos: Option<&Qos>,
    ) -> DdsResult<Self> {
        Self::with_qos_and_listener(publisher, topic, qos, None)
    }

    pub fn with_listener(
        publisher: dds_entity_t,
        topic: dds_entity_t,
        listener: &Listener,
    ) -> DdsResult<Self> {
        Self::with_qos_and_listener(publisher, topic, None, Some(listener))
    }

    pub fn with_qos_and_listener(
        publisher: dds_entity_t,
        topic: dds_entity_t,
        qos: Option<&Qos>,
        listener: Option<&Listener>,
    ) -> DdsResult<Self> {
        unsafe {
            let q = qos.map_or(std::ptr::null(), |q| q.as_ptr());
            let l = listener.map_or(std::ptr::null_mut(), |l| l.as_ptr());
            let handle = dds_create_writer(publisher, topic, q, l);
            check_entity(handle)?;
            Ok(DataWriter {
                entity: handle,
                _marker: PhantomData,
            })
        }
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, data)))]
    pub fn write(&self, data: &T) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe { check(dds_write(self.entity, ptr)) })
    }

    /// Write a sample with retry on transient errors.
    ///
    /// Retries up to `max_retries` times with exponential backoff
    /// starting at `base_delay_ms`.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, data)))]
    pub fn write_with_retry(
        &self,
        data: &T,
        max_retries: u32,
        base_delay_ms: u64,
    ) -> DdsResult<()> {
        let mut delay = std::time::Duration::from_millis(base_delay_ms);
        for attempt in 0..=max_retries {
            match self.write(data) {
                Ok(()) => return Ok(()),
                Err(e) if attempt < max_retries && e.is_transient() => {
                    std::thread::sleep(delay);
                    delay *= 2;
                }
                Err(e) => return Err(e),
            }
        }
        Err(crate::DdsError::Timeout)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, data)))]
    pub fn write_dispose(&self, data: &T) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe {
            check(dds_writedispose(self.entity, ptr))
        })
    }

    pub fn register_instance(&self, data: &T) -> DdsResult<dds_instance_handle_t> {
        self.with_native_ptr(data, |ptr| unsafe {
            let mut handle: dds_instance_handle_t = 0;
            check(dds_register_instance(self.entity, &mut handle, ptr))?;
            Ok(handle)
        })
    }

    pub fn unregister_instance(&self, data: &T) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe {
            check(dds_unregister_instance(self.entity, ptr))
        })
    }

    pub fn unregister_instance_handle(&self, handle: dds_instance_handle_t) -> DdsResult<()> {
        unsafe { check(dds_unregister_instance_ih(self.entity, handle)) }
    }

    pub fn dispose(&self, data: &T) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe { check(dds_dispose(self.entity, ptr)) })
    }

    pub fn dispose_instance_handle(&self, handle: dds_instance_handle_t) -> DdsResult<()> {
        unsafe { check(dds_dispose_ih(self.entity, handle)) }
    }

    pub fn lookup_instance(&self, data: &T) -> dds_instance_handle_t {
        self.with_native_ptr(data, |ptr| unsafe {
            Ok(dds_lookup_instance(self.entity, ptr))
        })
        .unwrap_or(0)
    }

    pub fn write_ts(&self, data: &T, timestamp: dds_time_t) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe {
            check(dds_write_ts(self.entity, ptr, timestamp))
        })
    }

    pub fn write_dispose_ts(&self, data: &T, timestamp: dds_time_t) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe {
            check(dds_writedispose_ts(self.entity, ptr, timestamp))
        })
    }

    pub fn write_flush(&self) -> DdsResult<()> {
        unsafe { check(dds_write_flush(self.entity)) }
    }

    pub fn wait_for_acks(&self, timeout: dds_duration_t) -> DdsResult<()> {
        unsafe { check(dds_wait_for_acks(self.entity, timeout)) }
    }

    pub fn assert_liveliness(&self) -> DdsResult<()> {
        unsafe { check(dds_assert_liveliness(self.entity)) }
    }

    /// Atualiza o QoS do writer em runtime (knobs online: TransportPriority,
    /// LatencyBudget, OwnershipStrength — o conjunto mutável avaliado pelo
    /// decisor de QoS, ver `dds-contract::qos::OnlineKnobs`).
    pub fn set_qos(&self, qos: &crate::Qos) -> DdsResult<()> {
        unsafe { check(dds_set_qos(self.entity, qos.as_ptr())) }
    }

    pub fn unregister_instance_ts(&self, data: &T, timestamp: dds_time_t) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe {
            check(dds_unregister_instance_ts(self.entity, ptr, timestamp))
        })
    }

    pub fn unregister_instance_handle_ts(
        &self,
        handle: dds_instance_handle_t,
        timestamp: dds_time_t,
    ) -> DdsResult<()> {
        unsafe {
            check(dds_unregister_instance_ih_ts(
                self.entity,
                handle,
                timestamp,
            ))
        }
    }

    pub fn dispose_ts(&self, data: &T, timestamp: dds_time_t) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe {
            check(dds_dispose_ts(self.entity, ptr, timestamp))
        })
    }

    pub fn dispose_instance_handle_ts(
        &self,
        handle: dds_instance_handle_t,
        timestamp: dds_time_t,
    ) -> DdsResult<()> {
        unsafe { check(dds_dispose_ih_ts(self.entity, handle, timestamp)) }
    }

    // ── Raw CDR write (Part 1.2) ──

    /// Write a sample that has already been serialized to CDR bytes.
    ///
    /// The `data` slice must contain a valid CDR encoding (including the
    /// encoding header) that matches the topic type of this writer.  The
    /// bytes are deserialized into a native sample and then written.
    ///
    /// For a more efficient zero-copy CDR write path, use `dds_writecdr`
    /// directly through the FFI layer.
    pub fn write_cdr(&self, data: &[u8]) -> DdsResult<()> {
        // Deserialize the CDR bytes into a native sample, then write it.
        // This is the safe and portable approach.
        //
        // We try XCDR2 first (for appendable/mutable types), then fall back
        // to XCDR1.
        let sample = crate::serialization::CdrDeserializer::<T>::deserialize(
            data,
            crate::serialization::CdrEncoding::Xcdr2,
        )
        .or_else(|_| {
            crate::serialization::CdrDeserializer::<T>::deserialize(
                data,
                crate::serialization::CdrEncoding::Xcdr1,
            )
        })?;

        self.write(&sample)
    }

    // ── Request Loan for zero-copy writes (Part 1.3) ──

    /// Request a loaned sample buffer from DDS for zero-copy writing.
    ///
    /// Returns a [`WriteLoan`] that holds a pointer to the loaned memory, typed
    /// as [`DdsType::Native`] — the DDS wire-compatible representation of `T`
    /// (for types with `String`/`Vec` fields this is a distinct, smaller struct
    /// with `DdsString`/`DdsSequence` fields; see the trait docs). Populate the
    /// sample via `loan.get_mut()`, then call [`WriteLoan::write`] to publish
    /// it.
    ///
    /// If the loan is dropped without being written, it is automatically
    /// returned to DDS without being published (and any fields already
    /// populated, e.g. a `DdsString`, are dropped correctly first).
    ///
    /// # SAFETY (history)
    ///
    /// Earlier versions of this method zero-initialized and cast the loaned
    /// buffer as `*mut T` directly. That was doubly unsound whenever `T` had
    /// heap-allocated fields: (1) `dds_request_loan` allocates exactly
    /// `T::descriptor_size()` bytes — `size_of::<T::Native>()` — which is
    /// *smaller* than `size_of::<T>()` once `String`(24B)/`Vec` fields are
    /// swapped for `DdsString`(8B)/`DdsSequence`, so zero-initializing
    /// `size_of::<T>()` bytes wrote past the end of the allocation (heap
    /// buffer overflow on every loan, regardless of whether the caller ever
    /// touched a string field); and (2) even with the size fixed, a zeroed
    /// `String`/`Vec` is not a valid bit-pattern, so assigning through
    /// `&mut T` would run `Drop` on an invalid value. Operating on
    /// `T::Native` (whose zero bit-pattern is valid by construction —
    /// `DdsString`/`DdsSequence`'s null/empty state — and whose size matches
    /// exactly what was allocated) fixes both issues. Found while
    /// implementing zero-copy writes for `tese/src/rust`'s `TaskOutput` (3
    /// `String` fields) — see that repo's `OPTIMIZATION_PLAN.md` Fase 4 for
    /// the full writeup.
    ///
    /// Known remaining gap (not a regression, pre-existing and out of scope
    /// for this fix): the derive macro's handling of a `Vec<Composite>` field
    /// uses the inner composite type directly as `DdsSequence<Inner>`'s
    /// element type rather than `<Inner as DdsType>::Native`. This is correct
    /// only when `Inner` has no heap fields of its own (true for every type
    /// this crate currently derives that use nested `Vec<Composite>`). A
    /// nested composite *with* `String`/`Vec` fields inside a `Vec<..>` would
    /// need the same treatment as this fix, recursively.
    pub fn request_loan(&self) -> DdsResult<WriteLoan<T>> {
        unsafe {
            let mut sample_ptr: *mut c_void = std::ptr::null_mut();
            check(dds_request_loan(self.entity, &mut sample_ptr))?;
            if sample_ptr.is_null() {
                return Err(crate::DdsError::OutOfResources);
            }
            // Zero-initialize exactly `size_of::<T::Native>()` bytes — this is
            // what CycloneDDS actually allocated (topic m_size ==
            // T::descriptor_size() == size_of::<T::Native>() by construction)
            // and `T::Native`'s all-zero state is a valid value (DdsString's
            // null pointer, DdsSequence's empty/unreleased state, or plain
            // zero-valid primitives).
            std::ptr::write_bytes(sample_ptr as *mut u8, 0, std::mem::size_of::<T::Native>());
            Ok(WriteLoan {
                sample: sample_ptr as *mut T::Native,
                writer: self.entity,
                written: false,
                _marker: PhantomData,
            })
        }
    }

    pub fn matched_subscriptions(&self) -> DdsResult<Vec<dds_instance_handle_t>> {
        unsafe {
            let count = dds_get_matched_subscriptions(self.entity, std::ptr::null_mut(), 0);
            if count < 0 {
                return Err(crate::DdsError::from(count));
            }
            let count = count as usize;
            if count == 0 {
                return Ok(Vec::new());
            }

            let mut handles = vec![0; count];
            let actual =
                dds_get_matched_subscriptions(self.entity, handles.as_mut_ptr(), handles.len());
            if actual < 0 {
                return Err(crate::DdsError::from(actual));
            }
            handles.truncate(actual as usize);
            Ok(handles)
        }
    }

    pub fn matched_subscription_endpoints(&self) -> DdsResult<Vec<MatchedEndpoint>> {
        let handles = self.matched_subscriptions()?;
        handles
            .into_iter()
            .map(|handle| MatchedEndpoint::from_subscription(self.entity, handle))
            .collect()
    }

    /// Get detailed endpoint data for a specific matched subscription.
    pub fn get_matched_subscription_data(
        &self,
        handle: dds_instance_handle_t,
    ) -> DdsResult<MatchedEndpoint> {
        MatchedEndpoint::from_subscription(self.entity, handle)
    }

    /// Serialize a sample to CDR bytes without writing.
    ///
    /// Convenience wrapper around [`CdrSerializer::serialize`] using XCDR1.
    pub fn serialize(&self, data: &T) -> DdsResult<Vec<u8>> {
        CdrSerializer::serialize(data, crate::serialization::CdrEncoding::Xcdr1)
    }

    /// Serialize a sample to CDR bytes with the specified encoding.
    pub fn serialize_with_encoding(
        &self,
        data: &T,
        encoding: crate::serialization::CdrEncoding,
    ) -> DdsResult<Vec<u8>> {
        CdrSerializer::serialize(data, encoding)
    }

    /// Request a loaned sample buffer and write it asynchronously.
    ///
    /// The `f` closure receives `&mut T::Native` (the wire-compatible
    /// representation — see [`DdsType::Native`]), allowing the caller to
    /// populate it in place. The sample is then published in a single
    /// zero-copy operation.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use cyclonedds::*;
    ///
    /// # #[derive(DdsTypeDerive)]
    /// # struct HelloWorld { id: i32, message: DdsString }
    /// # async fn example(writer: &DataWriter<HelloWorld>) -> DdsResult<()> {
    /// writer.write_loan_async(|sample| {
    ///     sample.id = 42;
    /// }).await
    /// # }
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument(skip(self, f)))]
    pub async fn write_loan_async<F>(&self, f: F) -> DdsResult<()>
    where
        F: FnOnce(&mut T::Native),
    {
        let mut loan = self.request_loan()?;
        f(loan.get_mut());
        WriteLoan::write(loan)
    }
}

impl<T: DdsType> DdsEntity for DataWriter<T> {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl<T: DdsType> Drop for DataWriter<T> {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}

// ---------------------------------------------------------------------------
// WriteLoan  (Part 1.3)
// ---------------------------------------------------------------------------

/// A loaned sample buffer for zero-copy writing.
///
/// Obtain one via [`DataWriter::request_loan`], populate it via
/// [`get_mut`](WriteLoan::get_mut) — which gives you `&mut T::Native`, the
/// wire-compatible representation, not `&mut T` — then call
/// [`write`](WriteLoan::write) to publish it.
///
/// If dropped without calling `write`, any fields already populated (e.g. a
/// `DdsString`) are dropped correctly, and the loan is returned to DDS (the
/// sample is *not* published).
pub struct WriteLoan<T: DdsType> {
    sample: *mut T::Native,
    writer: dds_entity_t,
    written: bool,
    _marker: PhantomData<T>,
}

impl<T: DdsType> WriteLoan<T> {
    /// Get a mutable reference to the loaned sample so you can populate it.
    ///
    /// This is `&mut T::Native` (the wire-compatible representation), not
    /// `&mut T` — for a type with `String` fields, populate the corresponding
    /// `DdsString` field via `DdsString::new(..)` instead of assigning a
    /// `String` directly.
    ///
    /// # Safety contract
    ///
    /// The buffer starts zero-initialized (a valid `T::Native` value — see
    /// [`DataWriter::request_loan`]). Normal field assignment
    /// (`loan.get_mut().field = value;`) is sound: it drops the old
    /// (zero-valid) field before moving in the new one. Do not move the
    /// entire referenced value out of the loan (the DDS loan owns this
    /// memory; only individual fields should be replaced).
    pub fn get_mut(&mut self) -> &mut T::Native {
        unsafe { &mut *self.sample }
    }

    /// Consume the loan and publish the sample.
    ///
    /// On success the loan is transferred to DDS (no copy needed) — DDS now
    /// owns the buffer and any heap-allocated fields it holds.
    /// On failure the loan is still consumed and the data is discarded.
    pub fn write(mut loan: Self) -> DdsResult<()> {
        loan.written = true;
        unsafe {
            let ret = dds_write(loan.writer, loan.sample as *const c_void);
            check(ret)
        }
    }
}

impl<T: DdsType> Drop for WriteLoan<T> {
    fn drop(&mut self) {
        if !self.written && !self.sample.is_null() {
            unsafe {
                // Run T::Native's destructor first: any DdsString/DdsSequence
                // field the caller already populated before abandoning the
                // loan owns real CycloneDDS-allocated memory that must be
                // freed here (a no-op for still-zeroed fields, since
                // DdsString/DdsSequence's Drop checks for the null/unreleased
                // state). Skipping this would leak.
                std::ptr::drop_in_place(self.sample);
                // Return the loan buffer itself.  dds_return_loan expects
                // a *mut *mut c_void array.
                let mut ptr = self.sample as *mut c_void;
                let _ = dds_return_loan(self.writer, &mut ptr, 1);
            }
        }
    }
}
