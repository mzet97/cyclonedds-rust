//! DataReader wrapper

use std::marker::PhantomData;
use std::ptr;
use cyclonedds_sys::*;
use crate::{DdsError, DdsResult};

/// A DataReader receives data samples of a specific type
pub struct DataReader<T> {
    entity: dds_entity_t,
    _marker: PhantomData<T>,
}

impl<T> DataReader<T> {
    /// Create a new DataReader
    pub fn new(subscriber: dds_entity_t, topic: dds_entity_t) -> DdsResult<Self> {
        unsafe {
            let reader = dds_create_reader(
                subscriber,
                topic,
                std::ptr::null(),
                std::ptr::null(),
            );

            if reader < 0 {
                return Err(DdsError::from(reader));
            }

            Ok(DataReader {
                entity: reader,
                _marker: PhantomData,
            })
        }
    }

    /// Read available data samples
    /// Note: This is a simplified implementation.
    /// Real implementation would need proper deserialization.
    pub fn read(&self) -> DdsResult<Vec<T>> {
        unsafe {
            let mut samples: *mut std::ffi::c_void = ptr::null_mut();
            let mut sample_info: *mut dds_sample_info_t = ptr::null_mut();
            let max_samples = 256;

            let n = dds_read(
                self.entity,
                &mut samples as *mut *mut std::ffi::c_void,
                &mut sample_info,
                0,
                max_samples,
                DDS_READ_SAMPLE_STATE_ANY,
                DDS_VIEW_STATE_ANY,
                DDS_ANY_INSTANCE_STATE,
            );

            if n < 0 {
                return Err(DdsError::from(n));
            }

            // Note: Proper deserialization would be needed here
            let _data: Vec<T> = Vec::new();

            // Clean up - in real impl we'd need to free samples
            // dds_return_loan(self.entity, samples, sample_info);

            Ok(_data)
        }
    }

    /// Take data samples (removes them from the queue)
    pub fn take(&self) -> DdsResult<Vec<T>> {
        unsafe {
            let mut samples: *mut std::ffi::c_void = ptr::null_mut();
            let mut sample_info: *mut dds_sample_info_t = ptr::null_mut();
            let max_samples = 256;

            let n = dds_take(
                self.entity,
                &mut samples as *mut *mut std::ffi::c_void,
                &mut sample_info,
                0,
                max_samples,
                DDS_READ_SAMPLE_STATE_ANY,
                DDS_VIEW_STATE_ANY,
                DDS_ANY_INSTANCE_STATE,
            );

            if n < 0 {
                return Err(DdsError::from(n));
            }

            // Note: Proper deserialization would be needed here
            let _data: Vec<T> = Vec::new();

            Ok(_data)
        }
    }

    /// Wait for data to become available
    /// Note: This is a placeholder - proper implementation would use waitset
    pub fn wait(&self, timeout_ms: u64) -> DdsResult<bool> {
        // For now, just return true to indicate potential data
        // Real implementation would use dds_waitset_* functions
        let _ = timeout_ms;
        Ok(true)
    }

    /// Get the underlying DDS entity handle
    pub fn as_entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl<T> Drop for DataReader<T> {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
