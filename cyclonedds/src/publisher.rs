use crate::{
    entity::{DdsEntity, OwnedEntity, OwnedHandle},
    error::{check, check_entity},
    DataWriter, DdsResult, DomainParticipant, Listener, Qos, Topic,
};
use cyclonedds_rust_sys::*;
use std::sync::Arc;

pub struct Publisher {
    inner: Arc<OwnedEntity>,
}

impl Publisher {
    /// Create a publisher under `participant`.
    ///
    /// The publisher holds the participant alive for as long as it lives, so it
    /// may outlive the binding it was created from.
    pub fn new(participant: &DomainParticipant) -> DdsResult<Self> {
        Self::with_qos_and_listener(participant, None, None)
    }

    pub fn with_qos(participant: &DomainParticipant, qos: Option<&Qos>) -> DdsResult<Self> {
        Self::with_qos_and_listener(participant, qos, None)
    }

    pub fn with_listener(participant: &DomainParticipant, listener: &Listener) -> DdsResult<Self> {
        Self::with_qos_and_listener(participant, None, Some(listener))
    }

    pub fn with_qos_and_listener(
        participant: &DomainParticipant,
        qos: Option<&Qos>,
        listener: Option<&Listener>,
    ) -> DdsResult<Self> {
        unsafe {
            let q = qos.map_or(std::ptr::null(), |q| q.as_ptr());
            let l = listener.map_or(std::ptr::null_mut(), |l| l.as_ptr());
            let handle = dds_create_publisher(participant.entity(), q, l);
            check_entity(handle)?;
            Ok(Publisher {
                inner: OwnedEntity::new(
                    handle,
                    "Publisher",
                    listener.cloned(),
                    vec![participant.owned().clone()],
                ),
            })
        }
    }

    /// Create a publisher under a raw participant handle.
    ///
    /// # Safety
    ///
    /// Nothing keeps the participant alive: if it is deleted first, CycloneDDS
    /// deletes this publisher with it and every call on it fails. Prefer
    /// [`Publisher::new`]; this exists for FFI interop, where the participant is
    /// owned elsewhere.
    pub unsafe fn from_entity(participant: dds_entity_t) -> DdsResult<Self> {
        unsafe { Self::from_entity_with(participant, None, None) }
    }

    /// [`Publisher::from_entity`] with QoS and a listener.
    ///
    /// # Safety
    ///
    /// The participant handle must remain valid until the returned publisher
    /// and all of its children are dropped.
    pub unsafe fn from_entity_with(
        participant: dds_entity_t,
        qos: Option<&Qos>,
        listener: Option<&Listener>,
    ) -> DdsResult<Self> {
        unsafe {
            let q = qos.map_or(std::ptr::null(), |q| q.as_ptr());
            let l = listener.map_or(std::ptr::null_mut(), |l| l.as_ptr());
            let handle = dds_create_publisher(participant, q, l);
            check_entity(handle)?;
            Ok(Publisher {
                inner: OwnedEntity::new(handle, "Publisher", listener.cloned(), Vec::new()),
            })
        }
    }

    pub fn create_writer<T: crate::DdsType>(&self, topic: &Topic<T>) -> DdsResult<DataWriter<T>> {
        DataWriter::new(self, topic)
    }

    pub fn create_writer_with_qos<T: crate::DdsType>(
        &self,
        topic: &Topic<T>,
        qos: &Qos,
    ) -> DdsResult<DataWriter<T>> {
        DataWriter::with_qos(self, topic, Some(qos))
    }

    pub fn begin_coherent(&self) -> DdsResult<()> {
        unsafe { check(dds_begin_coherent(self.entity())) }
    }

    pub fn end_coherent(&self) -> DdsResult<()> {
        unsafe { check(dds_end_coherent(self.entity())) }
    }

    pub fn suspend(&self) -> DdsResult<()> {
        unsafe { check(dds_suspend(self.entity())) }
    }

    pub fn resume(&self) -> DdsResult<()> {
        unsafe { check(dds_resume(self.entity())) }
    }
}

impl DdsEntity for Publisher {
    fn entity(&self) -> dds_entity_t {
        self.inner.handle()
    }
}

impl OwnedHandle for Publisher {
    fn owned(&self) -> &Arc<OwnedEntity> {
        &self.inner
    }
}
