use crate::{
    entity::{DdsEntity, OwnedEntity, OwnedHandle},
    error::{check, check_entity},
    DataReader, DdsResult, DomainParticipant, Listener, Qos, Topic,
};
use cyclonedds_rust_sys::*;
use std::sync::Arc;

pub struct Subscriber {
    inner: Arc<OwnedEntity>,
}

impl Subscriber {
    /// Create a subscriber under `participant`.
    ///
    /// The subscriber holds the participant alive for as long as it lives, so it
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
            let handle = dds_create_subscriber(participant.entity(), q, l);
            check_entity(handle)?;
            Ok(Subscriber {
                inner: OwnedEntity::new(
                    handle,
                    "Subscriber",
                    listener.cloned(),
                    vec![participant.owned().clone()],
                ),
            })
        }
    }

    /// Create a subscriber under a raw participant handle.
    ///
    /// # Safety
    ///
    /// Nothing keeps the participant alive: if it is deleted first, CycloneDDS
    /// deletes this subscriber with it and every call on it fails. Prefer
    /// [`Subscriber::new`]; this exists for FFI interop, where the participant is
    /// owned elsewhere.
    pub unsafe fn from_entity(participant: dds_entity_t) -> DdsResult<Self> {
        unsafe { Self::from_entity_with(participant, None, None) }
    }

    /// [`Subscriber::from_entity`] with QoS and a listener.
    ///
    /// # Safety
    ///
    /// The participant handle must remain valid until the returned subscriber
    /// and all of its children are dropped.
    pub unsafe fn from_entity_with(
        participant: dds_entity_t,
        qos: Option<&Qos>,
        listener: Option<&Listener>,
    ) -> DdsResult<Self> {
        unsafe {
            let q = qos.map_or(std::ptr::null(), |q| q.as_ptr());
            let l = listener.map_or(std::ptr::null_mut(), |l| l.as_ptr());
            let handle = dds_create_subscriber(participant, q, l);
            check_entity(handle)?;
            Ok(Subscriber {
                inner: OwnedEntity::new(handle, "Subscriber", listener.cloned(), Vec::new()),
            })
        }
    }

    pub fn create_reader<T: crate::DdsType>(&self, topic: &Topic<T>) -> DdsResult<DataReader<T>> {
        DataReader::new(self, topic)
    }

    pub fn create_reader_with_qos<T: crate::DdsType>(
        &self,
        topic: &Topic<T>,
        qos: &Qos,
    ) -> DdsResult<DataReader<T>> {
        DataReader::with_qos(self, topic, Some(qos))
    }

    pub fn notify_readers(&self) -> DdsResult<()> {
        unsafe { check(dds_notify_readers(self.entity())) }
    }

    /// Begin coherent access on this subscriber.
    ///
    /// Coherent access groups a set of data changes so that they are
    /// made available to readers as an atomic set.
    pub fn begin_coherent(&self) -> DdsResult<()> {
        unsafe { check(dds_begin_coherent(self.entity())) }
    }

    /// End coherent access on this subscriber.
    ///
    /// Must be paired with a prior [`begin_coherent`](Self::begin_coherent) call.
    pub fn end_coherent(&self) -> DdsResult<()> {
        unsafe { check(dds_end_coherent(self.entity())) }
    }
}

impl DdsEntity for Subscriber {
    fn entity(&self) -> dds_entity_t {
        self.inner.handle()
    }
}

impl OwnedHandle for Subscriber {
    fn owned(&self) -> &Arc<OwnedEntity> {
        &self.inner
    }
}
