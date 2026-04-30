use crate::{
    entity::DdsEntity,
    error::{check, check_entity},
    DataWriter, DdsResult, Listener, Qos, Topic,
};
use cyclonedds_sys::*;

pub struct Publisher {
    entity: dds_entity_t,
}

impl Publisher {
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
            let handle = dds_create_publisher(participant, q, l);
            check_entity(handle)?;
            Ok(Publisher { entity: handle })
        }
    }

    pub fn create_writer<T: crate::DdsType>(&self, topic: &Topic<T>) -> DdsResult<DataWriter<T>> {
        DataWriter::new(self.entity, topic.entity())
    }

    pub fn create_writer_with_qos<T: crate::DdsType>(
        &self,
        topic: &Topic<T>,
        qos: &Qos,
    ) -> DdsResult<DataWriter<T>> {
        DataWriter::with_qos(self.entity, topic.entity(), Some(qos))
    }

    pub fn begin_coherent(&self) -> DdsResult<()> {
        unsafe { check(dds_begin_coherent(self.entity)) }
    }

    pub fn end_coherent(&self) -> DdsResult<()> {
        unsafe { check(dds_end_coherent(self.entity)) }
    }

    pub fn suspend(&self) -> DdsResult<()> {
        unsafe { check(dds_suspend(self.entity)) }
    }

    pub fn resume(&self) -> DdsResult<()> {
        unsafe { check(dds_resume(self.entity)) }
    }
}

impl DdsEntity for Publisher {
    fn entity(&self) -> dds_entity_t {
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
