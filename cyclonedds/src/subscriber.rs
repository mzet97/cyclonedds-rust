//! Subscriber wrapper

use std::marker::PhantomData;
use cyclonedds_sys::*;
use crate::{DdsError, DdsResult, DataReader, Topic};

/// A Subscriber is responsible for creating and managing DataReaders
pub struct Subscriber {
    entity: dds_entity_t,
}

impl Subscriber {
    /// Create a new Subscriber
    pub fn new(participant: dds_entity_t) -> DdsResult<Self> {
        unsafe {
            let subscriber = dds_create_subscriber(
                participant,
                std::ptr::null(),
                std::ptr::null(),
            );

            if subscriber < 0 {
                return Err(DdsError::from(subscriber));
            }

            Ok(Subscriber { entity: subscriber })
        }
    }

    /// Create a DataReader for a topic
    pub fn create_reader<T: 'static>(&self, topic: &Topic<T>) -> DdsResult<DataReader<T>> {
        DataReader::new(self.entity, topic.as_entity())
    }

    /// Get the underlying DDS entity handle
    pub fn as_entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl Drop for Subscriber {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
