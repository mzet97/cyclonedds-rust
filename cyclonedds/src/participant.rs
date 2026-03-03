//! DomainParticipant wrapper

use std::ffi::CString;
use std::marker::PhantomData;
use cyclonedds_sys::*;
use crate::{DdsError, DdsResult, Topic, Publisher, Subscriber};

/// A DomainParticipant is the entry point for creating DDS entities
/// for a specific domain.
///
/// # Example
///
/// ```
/// use cyclonedds::DomainParticipant;
///
/// let participant = DomainParticipant::new(0).expect("Failed to create participant");
/// ```
pub struct DomainParticipant {
    entity: dds_entity_t,
}

impl DomainParticipant {
    /// Create a new DomainParticipant for the given domain ID
    pub fn new(domain_id: u32) -> DdsResult<Self> {
        unsafe {
            let participant = dds_create_participant(
                domain_id,
                std::ptr::null(),
                std::ptr::null(),
            );

            if participant < 0 {
                return Err(DdsError::from(participant));
            }

            Ok(DomainParticipant { entity: participant })
        }
    }

    /// Get the underlying DDS entity handle
    pub fn as_entity(&self) -> dds_entity_t {
        self.entity
    }

    /// Create a Topic for type T
    pub fn create_topic<T: 'static>(
        &self,
        name: &str,
    ) -> DdsResult<Topic<T>> {
        Topic::new(self.entity, name)
    }

    /// Create a Publisher
    pub fn create_publisher(&self) -> DdsResult<Publisher> {
        Publisher::new(self.entity)
    }

    /// Create a Subscriber
    pub fn create_subscriber(&self) -> DdsResult<Subscriber> {
        Subscriber::new(self.entity)
    }
}

impl Drop for DomainParticipant {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
