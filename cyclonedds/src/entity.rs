use crate::{
    error::{check, check_entity},
    statistics::Statistics,
    status::EntityStatus,
    xtypes::{SertypeHandle, TypeInfo, TypeObject},
    DdsError, DdsResult, Qos,
};
use cyclonedds_rust_sys::*;

pub trait DdsEntity {
    fn entity(&self) -> dds_entity_t;

    fn get_parent(&self) -> DdsResult<dds_entity_t> {
        unsafe { check_entity(dds_get_parent(self.entity())) }
    }

    fn get_participant(&self) -> DdsResult<dds_entity_t> {
        unsafe { check_entity(dds_get_participant(self.entity())) }
    }

    fn get_publisher(&self) -> DdsResult<dds_entity_t> {
        unsafe { check_entity(dds_get_publisher(self.entity())) }
    }

    fn get_subscriber(&self) -> DdsResult<dds_entity_t> {
        unsafe { check_entity(dds_get_subscriber(self.entity())) }
    }

    fn get_datareader(&self) -> DdsResult<dds_entity_t> {
        unsafe { check_entity(dds_get_datareader(self.entity())) }
    }

    fn get_topic(&self) -> DdsResult<dds_entity_t> {
        unsafe { check_entity(dds_get_topic(self.entity())) }
    }

    fn get_children(&self) -> DdsResult<Vec<dds_entity_t>> {
        unsafe {
            let mut buf: Vec<dds_entity_t> = vec![0; 64];
            let n = dds_get_children(self.entity(), buf.as_mut_ptr(), buf.len());
            if n < 0 {
                return Err(DdsError::from(n));
            }
            buf.truncate(n as usize);
            Ok(buf)
        }
    }

    fn get_name(&self) -> DdsResult<String> {
        unsafe {
            let mut buf = vec![0u8; 256];
            let n = dds_get_name(self.entity(), buf.as_mut_ptr() as *mut i8, buf.len());
            if n < 0 {
                return Err(DdsError::from(n));
            }
            buf.truncate(n as usize);
            Ok(String::from_utf8_lossy(&buf).into_owned())
        }
    }

    fn get_type_name(&self) -> DdsResult<String> {
        unsafe {
            let mut buf = vec![0u8; 256];
            let n = dds_get_type_name(self.entity(), buf.as_mut_ptr() as *mut i8, buf.len());
            if n < 0 {
                return Err(DdsError::from(n));
            }
            buf.truncate(n as usize);
            Ok(String::from_utf8_lossy(&buf).into_owned())
        }
    }

    fn get_domain_id(&self) -> DdsResult<u32> {
        unsafe {
            let mut id: dds_domainid_t = 0;
            check(dds_get_domainid(self.entity(), &mut id))?;
            Ok(id)
        }
    }

    fn get_instance_handle(&self) -> DdsResult<dds_instance_handle_t> {
        unsafe {
            let mut handle: dds_instance_handle_t = 0;
            check(dds_get_instance_handle(self.entity(), &mut handle))?;
            Ok(handle)
        }
    }

    fn get_guid(&self) -> DdsResult<dds_guid_t> {
        unsafe {
            let mut guid: dds_guid_t = std::mem::zeroed();
            check(dds_get_guid(self.entity(), &mut guid))?;
            Ok(guid)
        }
    }

    /// Convenience alias for [`Self::get_guid`].
    fn guid(&self) -> DdsResult<dds_guid_t> {
        self.get_guid()
    }

    /// Read a consolidated status snapshot for this entity.
    ///
    /// Each field is `Some` only when the corresponding status is supported
    /// by the entity type (e.g. a `DataWriter` will have
    /// `liveliness_lost` and `publication_matched`, but not `sample_lost`).
    fn status(&self) -> DdsResult<EntityStatus> {
        let mut s = EntityStatus::default();
        unsafe {
            let mut raw_it: dds_inconsistent_topic_status_t = std::mem::zeroed();
            if dds_get_inconsistent_topic_status(self.entity(), &mut raw_it) == 0 {
                s.inconsistent_topic = Some(raw_it.into());
            }
            let mut raw_ll: dds_liveliness_lost_status_t = std::mem::zeroed();
            if dds_get_liveliness_lost_status(self.entity(), &mut raw_ll) == 0 {
                s.liveliness_lost = Some(raw_ll.into());
            }
            let mut raw_lc: dds_liveliness_changed_status_t = std::mem::zeroed();
            if dds_get_liveliness_changed_status(self.entity(), &mut raw_lc) == 0 {
                s.liveliness_changed = Some(raw_lc.into());
            }
            let mut raw_odm: dds_offered_deadline_missed_status_t = std::mem::zeroed();
            if dds_get_offered_deadline_missed_status(self.entity(), &mut raw_odm) == 0 {
                s.offered_deadline_missed = Some(raw_odm.into());
            }
            let mut raw_oiq: dds_offered_incompatible_qos_status_t = std::mem::zeroed();
            if dds_get_offered_incompatible_qos_status(self.entity(), &mut raw_oiq) == 0 {
                s.offered_incompatible_qos = Some(raw_oiq.into());
            }
            let mut raw_rdm: dds_requested_deadline_missed_status_t = std::mem::zeroed();
            if dds_get_requested_deadline_missed_status(self.entity(), &mut raw_rdm) == 0 {
                s.requested_deadline_missed = Some(raw_rdm.into());
            }
            let mut raw_riq: dds_requested_incompatible_qos_status_t = std::mem::zeroed();
            if dds_get_requested_incompatible_qos_status(self.entity(), &mut raw_riq) == 0 {
                s.requested_incompatible_qos = Some(raw_riq.into());
            }
            let mut raw_sl: dds_sample_lost_status_t = std::mem::zeroed();
            if dds_get_sample_lost_status(self.entity(), &mut raw_sl) == 0 {
                s.sample_lost = Some(raw_sl.into());
            }
            let mut raw_sr: dds_sample_rejected_status_t = std::mem::zeroed();
            if dds_get_sample_rejected_status(self.entity(), &mut raw_sr) == 0 {
                s.sample_rejected = Some(raw_sr.into());
            }
            let mut raw_pm: dds_publication_matched_status_t = std::mem::zeroed();
            if dds_get_publication_matched_status(self.entity(), &mut raw_pm) == 0 {
                s.publication_matched = Some(raw_pm.into());
            }
            let mut raw_sm: dds_subscription_matched_status_t = std::mem::zeroed();
            if dds_get_subscription_matched_status(self.entity(), &mut raw_sm) == 0 {
                s.subscription_matched = Some(raw_sm.into());
            }
        }
        Ok(s)
    }

    fn enable(&self) -> DdsResult<()> {
        unsafe { check(dds_enable(self.entity())) }
    }

    fn get_status_mask(&self) -> DdsResult<u32> {
        unsafe {
            let mut mask: u32 = 0;
            check(dds_get_status_mask(self.entity(), &mut mask))?;
            Ok(mask)
        }
    }

    fn set_status_mask(&self, mask: u32) -> DdsResult<()> {
        unsafe { check(dds_set_status_mask(self.entity(), mask)) }
    }

    fn get_status_changes(&self) -> DdsResult<u32> {
        unsafe {
            let mut status: u32 = 0;
            check(dds_get_status_changes(self.entity(), &mut status))?;
            Ok(status)
        }
    }

    fn read_status(&self, mask: u32) -> DdsResult<u32> {
        unsafe {
            let mut status: u32 = 0;
            check(dds_read_status(self.entity(), &mut status, mask))?;
            Ok(status)
        }
    }

    fn take_status(&self, mask: u32) -> DdsResult<u32> {
        unsafe {
            let mut status: u32 = 0;
            check(dds_take_status(self.entity(), &mut status, mask))?;
            Ok(status)
        }
    }

    fn triggered(&self) -> DdsResult<bool> {
        unsafe {
            let ret = dds_triggered(self.entity());
            if ret < 0 {
                Err(DdsError::from(ret))
            } else {
                Ok(ret != 0)
            }
        }
    }

    fn is_shared_memory_available(&self) -> bool {
        unsafe { dds_is_shared_memory_available(self.entity()) }
    }

    fn create_statistics(&self) -> DdsResult<Statistics> {
        Statistics::new(self.entity())
    }

    fn get_qos(&self) -> DdsResult<Qos> {
        let mut qos = Qos::create()?;
        unsafe {
            check(dds_get_qos(self.entity(), qos.as_mut_ptr()))?;
        }
        Ok(qos)
    }

    fn get_sertype(&self) -> DdsResult<SertypeHandle> {
        unsafe {
            let mut ptr = std::ptr::null();
            check(dds_get_entity_sertype(self.entity(), &mut ptr))?;
            SertypeHandle::from_raw(ptr)
        }
    }

    fn get_type_info(&self) -> DdsResult<TypeInfo> {
        TypeInfo::from_entity(self.entity())
    }

    fn get_type_object(
        &self,
        type_id: *const dds_typeid_t,
        timeout: dds_duration_t,
    ) -> DdsResult<TypeObject> {
        TypeObject::from_entity_type_id(self.entity(), type_id, timeout)
    }

    fn get_minimal_type_object(&self, timeout: dds_duration_t) -> DdsResult<Option<TypeObject>> {
        self.get_type_info()?
            .minimal_type_object(self.entity(), timeout)
    }

    fn get_complete_type_object(&self, timeout: dds_duration_t) -> DdsResult<Option<TypeObject>> {
        self.get_type_info()?
            .complete_type_object(self.entity(), timeout)
    }

    fn matches_entity_type_info<E: DdsEntity>(&self, other: &E) -> DdsResult<bool> {
        Ok(self.get_type_info()?.matches(&other.get_type_info()?))
    }
}
