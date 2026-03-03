//! DataWriter wrapper

use std::marker::PhantomData;
use std::mem::MaybeUninit;
use cyclonedds_sys::*;
use crate::{DdsError, DdsResult};

/// A DataWriter publishes data samples of a specific type
pub struct DataWriter<T> {
    entity: dds_entity_t,
    _marker: PhantomData<T>,
}

impl<T> DataWriter<T> {
    /// Create a new DataWriter
    pub fn new(publisher: dds_entity_t, topic: dds_entity_t) -> DdsResult<Self> {
        unsafe {
            let writer = dds_create_writer(
                publisher,
                topic,
                std::ptr::null(),
                std::ptr::null(),
            );

            if writer < 0 {
                return Err(DdsError::from(writer));
            }

            Ok(DataWriter {
                entity: writer,
                _marker: PhantomData,
            })
        }
    }

    /// Write a data sample
    pub fn write(&self, data: &T) -> DdsResult<()> {
        unsafe {
            // Note: This is a simplified implementation.
            // Real implementation would need serde serialization
            // and proper memory management for the data.
            let data_ptr = data as *const T as *const std::ffi::c_void;

            let result = dds_write(self.entity, data_ptr);

            if result < 0 {
                return Err(DdsError::from(result));
            }

            Ok(())
        }
    }

    /// Get the underlying DDS entity handle
    pub fn as_entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl<T> Drop for DataWriter<T> {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
