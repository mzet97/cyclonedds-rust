use crate::{
    entity::DdsEntity,
    error::{check, check_entity},
    serialization::CdrSerializer,
    write_arena::WriteArena,
    xtypes::MatchedEndpoint,
    DdsResult, DdsType, Listener, Qos,
};
use cyclonedds_sys::*;
use std::ffi::c_void;
use std::marker::PhantomData;

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

    pub fn write(&self, data: &T) -> DdsResult<()> {
        self.with_native_ptr(data, |ptr| unsafe { check(dds_write(self.entity, ptr)) })
    }

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
    /// Returns a [`WriteLoan`] that holds a pointer to the loaned memory.
    /// Populate the sample via `loan.get_mut()`, then call
    /// [`WriteLoan::write`] to publish it.
    ///
    /// If the loan is dropped without being written, it is automatically
    /// returned to DDS without being published.
    pub fn request_loan(&self) -> DdsResult<WriteLoan<T>> {
        unsafe {
            let mut sample_ptr: *mut c_void = std::ptr::null_mut();
            check(dds_request_loan(self.entity, &mut sample_ptr))?;
            if sample_ptr.is_null() {
                return Err(crate::DdsError::OutOfResources);
            }
            // Zero-initialize the loaned memory so the caller starts from a
            // clean state (DDS does not guarantee the buffer contents).
            std::ptr::write_bytes(sample_ptr as *mut u8, 0, std::mem::size_of::<T>());
            Ok(WriteLoan {
                sample: sample_ptr as *mut T,
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
/// [`get_mut`](WriteLoan::get_mut), then call [`write`](WriteLoan::write) to
/// publish it.
///
/// If dropped without calling `write`, the loan is returned to DDS
/// (the sample is *not* published).
pub struct WriteLoan<T: DdsType> {
    sample: *mut T,
    writer: dds_entity_t,
    written: bool,
    _marker: PhantomData<T>,
}

impl<T: DdsType> WriteLoan<T> {
    /// Get a mutable reference to the loaned sample so you can populate it.
    ///
    /// # Safety contract
    ///
    /// The caller must not move out of the referenced value or replace it
    /// with one that contains heap allocations that expect to be dropped
    /// (the DDS loan owns the memory and `dds_return_loan` will free it).
    /// For plain-old-data and DDS-compatible types this is safe.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.sample }
    }

    /// Consume the loan and publish the sample.
    ///
    /// On success the loan is transferred to DDS (no copy needed).
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
            // Return the loan without writing.  dds_return_loan expects
            // a *mut *mut c_void array.
            unsafe {
                let mut ptr = self.sample as *mut c_void;
                let _ = dds_return_loan(self.writer, &mut ptr, 1);
            }
        }
    }
}
