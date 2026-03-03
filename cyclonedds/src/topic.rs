//! Topic wrapper

use std::ffi::CString;
use std::marker::PhantomData;
use cyclonedds_sys::*;
use crate::{DdsError, DdsResult};

/// A Topic is a typed channel for publishing and subscribing.
///
/// # Type Parameters
///
/// * `T` - The type being published/subscribed
pub struct Topic<T> {
    entity: dds_entity_t,
    _marker: PhantomData<T>,
}

impl<T> Topic<T> {
    /// Create a new Topic
    /// Note: type_name should be the name of the Rust type (e.g., "HelloWorld")
    pub fn new(participant: dds_entity_t, name: &str) -> DdsResult<Self> {
        unsafe {
            let topic_name = CString::new(name).map_err(|_| {
                DdsError::BadParameter("Topic name contains null byte".to_string())
            })?;

            // Use topic name as type name for now (simplified)
            let topic = dds_create_topic(
                participant,
                topic_name.as_ptr(),
                topic_name.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            );

            if topic < 0 {
                return Err(DdsError::from(topic));
            }

            Ok(Topic {
                entity: topic,
                _marker: PhantomData,
            })
        }
    }

    /// Get the underlying DDS entity handle
    pub fn as_entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl<T> Drop for Topic<T> {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
