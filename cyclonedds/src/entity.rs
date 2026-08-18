use crate::{
    error::{check, check_entity},
    statistics::Statistics,
    status::EntityStatus,
    xtypes::{SertypeHandle, TypeInfo, TypeObject},
    DdsError, DdsResult, Qos,
};
use cyclonedds_rust_sys::*;
use std::any::Any;
use std::sync::Mutex;

/// Read a string-valued entity property, growing the buffer if it did not fit.
///
/// `dds_get_name`/`dds_get_type_name` follow `snprintf` semantics: the return
/// value is the length the string *would* have needed, which can exceed the
/// buffer passed in. The previous code did `buf.truncate(n)` with that value —
/// and `Vec::truncate` only ever shrinks, so for a name of 256 bytes or more it
/// silently returned the whole raw buffer, embedded NUL padding included.
///
/// # Safety
/// `f` must be a CycloneDDS getter with `snprintf` return semantics that writes
/// a NUL-terminated string into the supplied buffer.
unsafe fn entity_string(
    entity: dds_entity_t,
    f: unsafe extern "C" fn(dds_entity_t, *mut i8, usize) -> dds_return_t,
) -> DdsResult<String> {
    let mut buf = vec![0u8; 256];
    let n = f(entity, buf.as_mut_ptr() as *mut i8, buf.len());
    if n < 0 {
        return Err(DdsError::from(n));
    }
    let needed = n as usize;
    if needed >= buf.len() {
        // Did not fit: retry once with an exactly-sized buffer (+1 for the NUL).
        buf = vec![0u8; needed + 1];
        let n = f(entity, buf.as_mut_ptr() as *mut i8, buf.len());
        if n < 0 {
            return Err(DdsError::from(n));
        }
    }
    buf.truncate(needed.min(buf.len()));
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

/// Delete an entity from a `Drop`, surfacing failure instead of discarding it.
///
/// With [`OwnedEntity`] holding every ancestor up, a failure here no longer
/// means "my parent deleted me first" — that ordering is now impossible by
/// construction. It means something genuinely unexpected, so it is still worth
/// reporting rather than discarding.
///
/// The raw constructors (`Topic::from_entity`, `Data{Reader,Writer}::from_entities`,
/// `Publisher::from_entity`, …) are the remaining way to reach the old failure:
/// they adopt a handle whose lifetime this crate does not control.
///
/// Never panics: a panicking `Drop` during an unwind aborts.
pub(crate) fn delete_entity(entity: dds_entity_t, what: &str) {
    let ret = unsafe { dds_delete(entity) };
    if ret < 0 {
        #[cfg(feature = "tracing")]
        tracing::warn!(
            entity,
            what,
            code = ret,
            "dds_delete failed during Drop; the handle was likely already \
             deleted by a parent entity (check field declaration order)"
        );
        #[cfg(all(debug_assertions, not(feature = "tracing")))]
        eprintln!(
            "cyclonedds: dds_delete({entity}) failed for {what} during Drop (code {ret}); \
             the handle was likely already deleted by a parent entity \
             (check field declaration order)"
        );
        #[cfg(all(not(debug_assertions), not(feature = "tracing")))]
        let _ = (entity, what);
    }
}

/// A `dds_entity_t` together with everything that has to outlive it.
///
/// CycloneDDS deletes an entity's entire subtree when the entity is deleted, so
/// a child handle is only valid while its parent is. Storing the handle bare —
/// which is what every wrapper in this crate used to do — made that invariant
/// depend on the order the *user* happened to declare their struct fields, and
/// fields drop in declaration order:
///
/// ```text
/// struct App { participant, subscriber, reader }   // participant dies first
/// ```
///
/// Every entity now holds an `Arc<OwnedEntity>` for each of its ancestors
/// instead. The parent's `dds_delete` cannot run until the last child has
/// released it, which makes declaration order irrelevant by construction, and
/// lets a child outlive the binding it was created from.
///
/// The field order below is the load-bearing part. `Drop::drop` runs before any
/// field is dropped, so the sequence is always: delete this entity → drop the
/// listener it could still have invoked → release the ancestors.
pub(crate) struct OwnedEntity {
    entity: dds_entity_t,
    what: &'static str,
    /// Held so the C callback data outlives the entity that can invoke it.
    _listener: Option<crate::Listener>,
    /// Ancestors, released only once this entity is gone.
    _parents: Vec<std::sync::Arc<OwnedEntity>>,
    /// Callback data retained for as long as this DDS entity or any child is alive.
    _callback_state: Mutex<Option<Box<dyn Any + Send + Sync>>>,
}

impl OwnedEntity {
    /// Adopt `entity` as a child of `parents`.
    pub(crate) fn new(
        entity: dds_entity_t,
        what: &'static str,
        listener: Option<crate::Listener>,
        parents: Vec<std::sync::Arc<OwnedEntity>>,
    ) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            entity,
            what,
            _listener: listener,
            _parents: parents,
            _callback_state: Mutex::new(None),
        })
    }

    /// Adopt a handle whose ancestors this crate does not hold.
    ///
    /// Only for the documented raw/FFI constructors: nothing keeps the parent
    /// alive, so the caller carries the invariant the rest of the crate now
    /// enforces.
    pub(crate) fn unowned(entity: dds_entity_t, what: &'static str) -> std::sync::Arc<Self> {
        Self::new(entity, what, None, Vec::new())
    }

    pub(crate) fn handle(&self) -> dds_entity_t {
        self.entity
    }

    /// The listener this entity is holding up, if any.
    #[cfg(test)]
    pub(crate) fn listener(&self) -> Option<&crate::Listener> {
        self._listener.as_ref()
    }

    /// Add an ancestor after construction.
    ///
    /// Only reachable through [`retaining`](crate::UntypedTopic::retaining),
    /// which requires the `Arc` to still be unique — i.e. the entity has not
    /// been handed to anyone yet.
    pub(crate) fn push_parent(&mut self, parent: std::sync::Arc<OwnedEntity>) {
        self._parents.push(parent);
    }

    pub(crate) fn replace_callback_state(&self, state: Option<Box<dyn Any + Send + Sync>>) {
        let mut current = self
            ._callback_state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *current = state;
    }
}

/// Crate-internal access to a wrapper's owned handle, so a constructor can take
/// its parent's `Arc` without every type needing a bespoke accessor.
pub(crate) trait OwnedHandle {
    fn owned(&self) -> &std::sync::Arc<OwnedEntity>;
}

impl Drop for OwnedEntity {
    fn drop(&mut self) {
        delete_entity(self.entity, self.what);
    }
}

pub trait DdsEntity {
    /// The raw CycloneDDS handle behind this wrapper.
    ///
    /// **Deliberately public**, and the decision is recorded rather than
    /// revisited: a review proposed making it `pub(crate)`, on the grounds that
    /// a raw handle is an invitation to misuse it. It stays public because the
    /// alternative is worse. Interoperating with C — passing an entity to a
    /// CycloneDDS API this crate does not wrap, or adopting one created
    /// elsewhere — is a supported use, and closing this would force every such
    /// caller into `unsafe` transmutes to recover a number the wrapper already
    /// has. The raw `from_entity`/`from_entities` constructors depend on it too.
    ///
    /// What the handle does *not* carry is ownership. Every wrapper holds an
    /// `Arc<OwnedEntity>` that keeps its ancestors alive; the `dds_entity_t`
    /// returned here is a plain integer with none of that. Two rules follow:
    ///
    /// * It is valid only while the wrapper it came from is alive. Storing it
    ///   past that point is the pre-3.0 defect this crate spent a release
    ///   fixing — CycloneDDS deletes an entity's whole subtree, and handles are
    ///   redrawn, so a stale one usually errors and occasionally does worse.
    /// * Do not call `dds_delete` on it. The wrapper's `Drop` will.
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
        unsafe { entity_string(self.entity(), dds_get_name) }
    }

    fn get_type_name(&self) -> DdsResult<String> {
        unsafe { entity_string(self.entity(), dds_get_type_name) }
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

    /// Get the set of triggered status bits that have changed since the
    /// last time they were read.
    fn status_changes(&self) -> DdsResult<u32> {
        unsafe {
            let mut changes: u32 = 0;
            check(dds_get_status_changes(self.entity(), &mut changes))?;
            Ok(changes)
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
            .minimal_type_object_from_entity(self.entity(), timeout)
    }

    fn get_complete_type_object(&self, timeout: dds_duration_t) -> DdsResult<Option<TypeObject>> {
        self.get_type_info()?
            .complete_type_object_from_entity(self.entity(), timeout)
    }

    fn matches_entity_type_info<E: DdsEntity>(&self, other: &E) -> DdsResult<bool> {
        Ok(self.get_type_info()?.matches(&other.get_type_info()?))
    }
}
