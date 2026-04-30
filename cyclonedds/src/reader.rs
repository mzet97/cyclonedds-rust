use crate::{
    entity::DdsEntity,
    error::check_entity,
    serialization::{CdrDeserializer, CdrEncoding, CdrSample},
    xtypes::MatchedEndpoint,
    DdsError, DdsResult, DdsType, Listener, Loan, Qos, Sample,
};
use cyclonedds_sys::*;
use std::marker::PhantomData;
use std::ptr;

pub struct DataReader<T: DdsType> {
    entity: dds_entity_t,
    _marker: PhantomData<T>,
}

impl<T: DdsType> DataReader<T> {
    pub fn new(subscriber: dds_entity_t, topic: dds_entity_t) -> DdsResult<Self> {
        Self::with_qos_and_listener(subscriber, topic, None, None)
    }

    pub fn with_qos(
        subscriber: dds_entity_t,
        topic: dds_entity_t,
        qos: Option<&Qos>,
    ) -> DdsResult<Self> {
        Self::with_qos_and_listener(subscriber, topic, qos, None)
    }

    pub fn with_listener(
        subscriber: dds_entity_t,
        topic: dds_entity_t,
        listener: &Listener,
    ) -> DdsResult<Self> {
        Self::with_qos_and_listener(subscriber, topic, None, Some(listener))
    }

    pub fn with_qos_and_listener(
        subscriber: dds_entity_t,
        topic: dds_entity_t,
        qos: Option<&Qos>,
        listener: Option<&Listener>,
    ) -> DdsResult<Self> {
        unsafe {
            let q = qos.map_or(std::ptr::null(), |q| q.as_ptr());
            let l = listener.map_or(std::ptr::null_mut(), |l| l.as_ptr());
            let handle = dds_create_reader(subscriber, topic, q, l);
            check_entity(handle)?;
            Ok(DataReader {
                entity: handle,
                _marker: PhantomData,
            })
        }
    }

    // ── Existing Vec<T>-returning methods (kept for backward compat) ──

    pub fn read(&self) -> DdsResult<Vec<T>> {
        self.read_impl(false)
    }

    pub fn take(&self) -> DdsResult<Vec<T>> {
        self.read_impl(true)
    }

    fn read_impl(&self, take: bool) -> DdsResult<Vec<T>> {
        unsafe {
            let max_samples: usize = 256;
            let mut samples: Vec<*mut std::ffi::c_void> = vec![ptr::null_mut(); max_samples];
            let mut infos: Vec<dds_sample_info> = vec![std::mem::zeroed(); max_samples];

            let n = if take {
                dds_take(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                )
            } else {
                dds_read(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                )
            };

            if n < 0 {
                return Err(DdsError::from(n));
            }
            let n = n as usize;

            let mut result = Vec::with_capacity(n);
            for i in 0..n {
                if infos[i].valid_data && !samples[i].is_null() {
                    let data = T::clone_out(samples[i] as *const T);
                    result.push(data);
                }
            }

            let _ = dds_return_loan(self.entity, samples.as_mut_ptr(), n as i32);
            Ok(result)
        }
    }

    // ── Zero-copy Loan<T>-returning methods ──

    /// Zero-copy read. Returns a `Loan<T>` that auto-returns the loan on drop.
    pub fn read_loan(&self) -> DdsResult<Loan<T>> {
        self.loan_impl(false, 0, 0, false, false)
    }

    /// Zero-copy take. Returns a `Loan<T>` that auto-returns the loan on drop.
    pub fn take_loan(&self) -> DdsResult<Loan<T>> {
        self.loan_impl(true, 0, 0, false, false)
    }

    /// Read samples for a specific instance handle (zero-copy).
    pub fn read_instance(&self, handle: dds_instance_handle_t) -> DdsResult<Loan<T>> {
        self.loan_impl(false, handle, 0, true, false)
    }

    /// Take samples for a specific instance handle (zero-copy).
    pub fn take_instance(&self, handle: dds_instance_handle_t) -> DdsResult<Loan<T>> {
        self.loan_impl(true, handle, 0, true, false)
    }

    /// Read samples matching the given state mask (zero-copy).
    pub fn read_mask(&self, mask: u32) -> DdsResult<Loan<T>> {
        self.loan_impl(false, 0, mask, false, true)
    }

    /// Take samples matching the given state mask (zero-copy).
    pub fn take_mask(&self, mask: u32) -> DdsResult<Loan<T>> {
        self.loan_impl(true, 0, mask, false, true)
    }

    /// Peek (non-consuming read) — does not change sample state.
    pub fn peek(&self) -> DdsResult<Loan<T>> {
        self.peek_impl(0, 0, false, false)
    }

    /// Peek for a specific instance handle.
    pub fn peek_instance(&self, handle: dds_instance_handle_t) -> DdsResult<Loan<T>> {
        self.peek_impl(handle, 0, true, false)
    }

    /// Peek with state mask filter.
    pub fn peek_mask(&self, mask: u32) -> DdsResult<Loan<T>> {
        self.peek_impl(0, mask, false, true)
    }

    fn loan_impl(
        &self,
        take: bool,
        handle: dds_instance_handle_t,
        mask: u32,
        use_instance: bool,
        use_mask: bool,
    ) -> DdsResult<Loan<T>> {
        unsafe {
            let max_samples: usize = 256;
            let mut samples: Vec<*mut std::ffi::c_void> = vec![ptr::null_mut(); max_samples];
            let mut infos: Vec<dds_sample_info_t> = vec![std::mem::zeroed(); max_samples];

            let n = if use_instance && take {
                dds_take_instance(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                    handle,
                )
            } else if use_instance {
                dds_read_instance(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                    handle,
                )
            } else if use_mask && take {
                dds_take_mask(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                    mask,
                )
            } else if use_mask {
                dds_read_mask(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                    mask,
                )
            } else if take {
                dds_take(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                )
            } else {
                dds_read(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                )
            };

            if n < 0 {
                return Err(DdsError::from(n));
            }
            let n = n as usize;

            Ok(Loan::new(samples, infos, n, self.entity))
        }
    }

    fn peek_impl(
        &self,
        handle: dds_instance_handle_t,
        mask: u32,
        use_instance: bool,
        use_mask: bool,
    ) -> DdsResult<Loan<T>> {
        unsafe {
            let max_samples: usize = 256;
            let mut samples: Vec<*mut std::ffi::c_void> = vec![ptr::null_mut(); max_samples];
            let mut infos: Vec<dds_sample_info_t> = vec![std::mem::zeroed(); max_samples];

            let n = if use_instance && use_mask {
                dds_peek_instance_mask(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                    handle,
                    mask,
                )
            } else if use_instance {
                dds_peek_instance(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                    handle,
                )
            } else if use_mask {
                dds_peek_mask(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                    mask,
                )
            } else {
                dds_peek(
                    self.entity,
                    samples.as_mut_ptr(),
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    max_samples,
                    max_samples as u32,
                )
            };

            if n < 0 {
                return Err(DdsError::from(n));
            }
            let n = n as usize;

            Ok(Loan::new(samples, infos, n, self.entity))
        }
    }

    /// Read the next unread sample (equivalent to `dds_read_next`).
    pub fn read_next(&self) -> DdsResult<Option<Sample<T>>> {
        unsafe {
            let mut sample: *mut std::ffi::c_void = ptr::null_mut();
            let mut info: dds_sample_info_t = std::mem::zeroed();
            let n = dds_read_next(
                self.entity,
                &mut sample,
                &mut info as *mut dds_sample_info_t,
            );
            if n < 0 {
                return Err(DdsError::from(n));
            }
            if n == 0 || !info.valid_data || sample.is_null() {
                let _ = dds_return_loan(self.entity, &mut sample as *mut _, 1);
                return Ok(None);
            }
            let data = T::clone_out(sample as *const T);
            let _ = dds_return_loan(self.entity, &mut sample as *mut _, 1);
            Ok(Some(Sample { data, info }))
        }
    }

    /// Take the next unread sample (equivalent to `dds_take_next`).
    pub fn take_next(&self) -> DdsResult<Option<Sample<T>>> {
        unsafe {
            let mut sample: *mut std::ffi::c_void = ptr::null_mut();
            let mut info: dds_sample_info_t = std::mem::zeroed();
            let n = dds_take_next(
                self.entity,
                &mut sample,
                &mut info as *mut dds_sample_info_t,
            );
            if n < 0 {
                return Err(DdsError::from(n));
            }
            if n == 0 || !info.valid_data || sample.is_null() {
                let _ = dds_return_loan(self.entity, &mut sample as *mut _, 1);
                return Ok(None);
            }
            let data = T::clone_out(sample as *const T);
            let _ = dds_return_loan(self.entity, &mut sample as *mut _, 1);
            Ok(Some(Sample { data, info }))
        }
    }

    // ── Instance management ──

    pub fn lookup_instance(&self, data: &T) -> dds_instance_handle_t {
        unsafe { dds_lookup_instance(self.entity, data as *const T as *const std::ffi::c_void) }
    }

    pub fn instance_get_key(&self, ih: dds_instance_handle_t) -> DdsResult<T> {
        unsafe {
            let mut data: T = std::mem::zeroed();
            let ret = dds_instance_get_key(
                self.entity,
                ih,
                &mut data as *mut T as *mut std::ffi::c_void,
            );
            check_entity(ret)?;
            Ok(data)
        }
    }

    pub fn wait_for_historical_data(&self, timeout: dds_duration_t) -> DdsResult<()> {
        unsafe { crate::error::check(dds_reader_wait_for_historical_data(self.entity, timeout)) }
    }

    // ── Raw CDR read/take (Part 1.2) ──

    /// Read samples as raw CDR bytes.
    ///
    /// Returns [`CdrSample`]s containing the serialized data and sample info.
    /// The samples remain in the reader history cache (state is updated).
    pub fn read_cdr(&self) -> DdsResult<Vec<CdrSample>> {
        self.cdr_impl(false, 0, 0, false, false)
    }

    /// Take samples as raw CDR bytes.
    ///
    /// Like [`read_cdr`](Self::read_cdr) but removes the samples from the
    /// reader history cache.
    pub fn take_cdr(&self) -> DdsResult<Vec<CdrSample>> {
        self.cdr_impl(true, 0, 0, false, false)
    }

    /// Read raw CDR samples for a specific instance.
    pub fn read_cdr_instance(&self, handle: dds_instance_handle_t) -> DdsResult<Vec<CdrSample>> {
        self.cdr_impl(false, handle, 0, true, false)
    }

    /// Take raw CDR samples for a specific instance.
    pub fn take_cdr_instance(&self, handle: dds_instance_handle_t) -> DdsResult<Vec<CdrSample>> {
        self.cdr_impl(true, handle, 0, true, false)
    }

    /// Read raw CDR samples matching the given state mask.
    pub fn read_cdr_mask(&self, mask: u32) -> DdsResult<Vec<CdrSample>> {
        self.cdr_impl(false, 0, mask, false, true)
    }

    /// Take raw CDR samples matching the given state mask.
    pub fn take_cdr_mask(&self, mask: u32) -> DdsResult<Vec<CdrSample>> {
        self.cdr_impl(true, 0, mask, false, true)
    }

    fn cdr_impl(
        &self,
        take: bool,
        handle: dds_instance_handle_t,
        mask: u32,
        use_instance: bool,
        use_mask: bool,
    ) -> DdsResult<Vec<CdrSample>> {
        unsafe {
            let max_samples: usize = 256;
            let mut buf: Vec<*mut ddsi_serdata> = vec![std::ptr::null_mut(); max_samples];
            let mut infos: Vec<dds_sample_info_t> = vec![std::mem::zeroed(); max_samples];

            let n = if use_instance && take {
                dds_takecdr_instance(
                    self.entity,
                    buf.as_mut_ptr(),
                    max_samples as u32,
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    handle,
                    mask,
                )
            } else if use_instance {
                dds_readcdr_instance(
                    self.entity,
                    buf.as_mut_ptr(),
                    max_samples as u32,
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    handle,
                    mask,
                )
            } else if use_mask && take {
                dds_takecdr(
                    self.entity,
                    buf.as_mut_ptr(),
                    max_samples as u32,
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    mask,
                )
            } else if use_mask {
                dds_readcdr(
                    self.entity,
                    buf.as_mut_ptr(),
                    max_samples as u32,
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    mask,
                )
            } else if take {
                dds_takecdr(
                    self.entity,
                    buf.as_mut_ptr(),
                    max_samples as u32,
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    0,
                )
            } else {
                dds_readcdr(
                    self.entity,
                    buf.as_mut_ptr(),
                    max_samples as u32,
                    infos.as_mut_ptr() as *mut dds_sample_info_t,
                    0,
                )
            };

            if n < 0 {
                return Err(DdsError::from(n));
            }
            let n = n as usize;

            let mut result = Vec::with_capacity(n);
            for i in 0..n {
                let sd = buf[i];
                if sd.is_null() || !infos[i].valid_data {
                    if !sd.is_null() {
                        ddsi_serdata_unref(sd);
                    }
                    continue;
                }

                let size = ddsi_serdata_size(sd) as usize;
                let mut data = vec![0u8; size];
                ddsi_serdata_to_ser(sd, 0, size, data.as_mut_ptr() as *mut std::ffi::c_void);

                ddsi_serdata_unref(sd);

                result.push(CdrSample {
                    data,
                    info: infos[i],
                });
            }

            Ok(result)
        }
    }

    /// Deserialize CDR bytes into a typed sample.
    ///
    /// Convenience wrapper around [`CdrDeserializer::deserialize`].
    pub fn deserialize_cdr(&self, data: &[u8], encoding: CdrEncoding) -> DdsResult<T> {
        CdrDeserializer::<T>::deserialize(data, encoding)
    }

    pub fn matched_publications(&self) -> DdsResult<Vec<dds_instance_handle_t>> {
        unsafe {
            let count = dds_get_matched_publications(self.entity, std::ptr::null_mut(), 0);
            if count < 0 {
                return Err(DdsError::from(count));
            }
            let count = count as usize;
            if count == 0 {
                return Ok(Vec::new());
            }

            let mut handles = vec![0; count];
            let actual =
                dds_get_matched_publications(self.entity, handles.as_mut_ptr(), handles.len());
            if actual < 0 {
                return Err(DdsError::from(actual));
            }
            handles.truncate(actual as usize);
            Ok(handles)
        }
    }

    pub fn matched_publication_endpoints(&self) -> DdsResult<Vec<MatchedEndpoint>> {
        let handles = self.matched_publications()?;
        handles
            .into_iter()
            .map(|handle| MatchedEndpoint::from_publication(self.entity, handle))
            .collect()
    }

    /// Get detailed endpoint data for a specific matched publication.
    pub fn get_matched_publication_data(
        &self,
        handle: dds_instance_handle_t,
    ) -> DdsResult<MatchedEndpoint> {
        MatchedEndpoint::from_publication(self.entity, handle)
    }
}

impl<T: DdsType> DdsEntity for DataReader<T> {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl<T: DdsType> Drop for DataReader<T> {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
