//! Publisher wrapper

use std::marker::PhantomData;
use cyclonedds_sys::*;
use crate::{DdsError, DdsResult, DataWriter, Topic};

/// A Publisher is responsible for creating and managing DataWriters
pub struct Publisher {
    entity: dds_entity_t,
}

impl Publisher {
    /// Create a new Publisher
    pub fn new(participant: dds_entity_t) -> DdsResult<Self> {
        unsafe {
            let publisher = dds_create_publisher(
                participant,
                std::ptr::null(),
                std::ptr::null(),
            );

            if publisher < 0 {
                return Err(DdsError::from(publisher));
            }

            Ok(Publisher { entity: publisher })
        }
    }

    /// Create a DataWriter for a topic
    pub fn create_writer<T: 'static>(&self, topic: &Topic<T>) -> DdsResult<DataWriter<T>> {
        DataWriter::new(self.entity, topic.as_entity())
    }

    /// Get the underlying DDS entity handle
    pub fn as_entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl Drop for Publisher {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
