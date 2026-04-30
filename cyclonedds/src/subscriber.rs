use crate::{
    entity::DdsEntity,
    error::{check, check_entity},
    DataReader, DdsResult, Listener, Qos, Topic,
};
use cyclonedds_rust_sys::*;

pub struct Subscriber {
    entity: dds_entity_t,
}

impl Subscriber {
    pub fn new(participant: dds_entity_t) -> DdsResult<Self> {
        Self::with_qos_and_listener(participant, None, None)
    }

    pub fn with_qos(participant: dds_entity_t, qos: Option<&Qos>) -> DdsResult<Self> {
        Self::with_qos_and_listener(participant, qos, None)
    }

    pub fn with_listener(participant: dds_entity_t, listener: &Listener) -> DdsResult<Self> {
        Self::with_qos_and_listener(participant, None, Some(listener))
    }

    pub fn with_qos_and_listener(
        participant: dds_entity_t,
        qos: Option<&Qos>,
        listener: Option<&Listener>,
    ) -> DdsResult<Self> {
        unsafe {
            let q = qos.map_or(std::ptr::null(), |q| q.as_ptr());
            let l = listener.map_or(std::ptr::null_mut(), |l| l.as_ptr());
            let handle = dds_create_subscriber(participant, q, l);
            check_entity(handle)?;
            Ok(Subscriber { entity: handle })
        }
    }

    pub fn create_reader<T: crate::DdsType>(&self, topic: &Topic<T>) -> DdsResult<DataReader<T>> {
        DataReader::new(self.entity, topic.entity())
    }

    pub fn create_reader_with_qos<T: crate::DdsType>(
        &self,
        topic: &Topic<T>,
        qos: &Qos,
    ) -> DdsResult<DataReader<T>> {
        DataReader::with_qos(self.entity, topic.entity(), Some(qos))
    }

    pub fn notify_readers(&self) -> DdsResult<()> {
        unsafe { check(dds_notify_readers(self.entity)) }
    }

    /// Begin coherent access on this subscriber.
    ///
    /// Coherent access groups a set of data changes so that they are
    /// made available to readers as an atomic set.
    pub fn begin_coherent(&self) -> DdsResult<()> {
        unsafe { check(dds_begin_coherent(self.entity)) }
    }

    /// End coherent access on this subscriber.
    ///
    /// Must be paired with a prior [`begin_coherent`](Self::begin_coherent) call.
    pub fn end_coherent(&self) -> DdsResult<()> {
        unsafe { check(dds_end_coherent(self.entity)) }
    }
}

impl DdsEntity for Subscriber {
    fn entity(&self) -> dds_entity_t {
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
