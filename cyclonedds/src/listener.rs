use crate::DdsResult;
use cyclonedds_sys::*;
use std::ffi::c_void;
use std::sync::Arc;

// Existing callback types (4)
type DataAvailableCb = Arc<dyn Fn(i32) + Send + Sync>;
type PublicationMatchedCb = Arc<dyn Fn(i32, dds_publication_matched_status_t) + Send + Sync>;
type SubscriptionMatchedCb = Arc<dyn Fn(i32, dds_subscription_matched_status_t) + Send + Sync>;
type LivelinessChangedCb = Arc<dyn Fn(i32, dds_liveliness_changed_status_t) + Send + Sync>;

// New callback types (9 with status, 1 without)
type InconsistentTopicCb = Arc<dyn Fn(i32, dds_inconsistent_topic_status_t) + Send + Sync>;
type LivelinessLostCb = Arc<dyn Fn(i32, dds_liveliness_lost_status_t) + Send + Sync>;
type OfferedDeadlineMissedCb = Arc<dyn Fn(i32, dds_offered_deadline_missed_status_t) + Send + Sync>;
type OfferedIncompatibleQosCb =
    Arc<dyn Fn(i32, dds_offered_incompatible_qos_status_t) + Send + Sync>;
type DataOnReadersCb = Arc<dyn Fn(i32) + Send + Sync>;
type SampleLostCb = Arc<dyn Fn(i32, dds_sample_lost_status_t) + Send + Sync>;
type SampleRejectedCb = Arc<dyn Fn(i32, dds_sample_rejected_status_t) + Send + Sync>;
type RequestedDeadlineMissedCb =
    Arc<dyn Fn(i32, dds_requested_deadline_missed_status_t) + Send + Sync>;
type RequestedIncompatibleQosCb =
    Arc<dyn Fn(i32, dds_requested_incompatible_qos_status_t) + Send + Sync>;

struct ListenerClosures {
    // Existing
    on_data_available: Option<DataAvailableCb>,
    on_publication_matched: Option<PublicationMatchedCb>,
    on_subscription_matched: Option<SubscriptionMatchedCb>,
    on_liveliness_changed: Option<LivelinessChangedCb>,
    // New
    on_inconsistent_topic: Option<InconsistentTopicCb>,
    on_liveliness_lost: Option<LivelinessLostCb>,
    on_offered_deadline_missed: Option<OfferedDeadlineMissedCb>,
    on_offered_incompatible_qos: Option<OfferedIncompatibleQosCb>,
    on_data_on_readers: Option<DataOnReadersCb>,
    on_sample_lost: Option<SampleLostCb>,
    on_sample_rejected: Option<SampleRejectedCb>,
    on_requested_deadline_missed: Option<RequestedDeadlineMissedCb>,
    on_requested_incompatible_qos: Option<RequestedIncompatibleQosCb>,
}

pub struct Listener {
    ptr: *mut dds_listener_t,
    _closures: Box<ListenerClosures>,
}

// ── Existing trampolines ────────────────────────────────────────────

unsafe extern "C" fn trampoline_data_available(reader: dds_entity_t, arg: *mut c_void) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_data_available {
        cb(reader);
    }
}

unsafe extern "C" fn trampoline_publication_matched(
    writer: dds_entity_t,
    status: dds_publication_matched_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_publication_matched {
        cb(writer, status);
    }
}

unsafe extern "C" fn trampoline_subscription_matched(
    reader: dds_entity_t,
    status: dds_subscription_matched_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_subscription_matched {
        cb(reader, status);
    }
}

unsafe extern "C" fn trampoline_liveliness_changed(
    reader: dds_entity_t,
    status: dds_liveliness_changed_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_liveliness_changed {
        cb(reader, status);
    }
}

// ── New trampolines ─────────────────────────────────────────────────

unsafe extern "C" fn trampoline_inconsistent_topic(
    topic: dds_entity_t,
    status: dds_inconsistent_topic_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_inconsistent_topic {
        cb(topic, status);
    }
}

unsafe extern "C" fn trampoline_liveliness_lost(
    writer: dds_entity_t,
    status: dds_liveliness_lost_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_liveliness_lost {
        cb(writer, status);
    }
}

unsafe extern "C" fn trampoline_offered_deadline_missed(
    writer: dds_entity_t,
    status: dds_offered_deadline_missed_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_offered_deadline_missed {
        cb(writer, status);
    }
}

unsafe extern "C" fn trampoline_offered_incompatible_qos(
    writer: dds_entity_t,
    status: dds_offered_incompatible_qos_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_offered_incompatible_qos {
        cb(writer, status);
    }
}

unsafe extern "C" fn trampoline_data_on_readers(subscriber: dds_entity_t, arg: *mut c_void) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_data_on_readers {
        cb(subscriber);
    }
}

unsafe extern "C" fn trampoline_sample_lost(
    reader: dds_entity_t,
    status: dds_sample_lost_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_sample_lost {
        cb(reader, status);
    }
}

unsafe extern "C" fn trampoline_sample_rejected(
    reader: dds_entity_t,
    status: dds_sample_rejected_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_sample_rejected {
        cb(reader, status);
    }
}

unsafe extern "C" fn trampoline_requested_deadline_missed(
    reader: dds_entity_t,
    status: dds_requested_deadline_missed_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_requested_deadline_missed {
        cb(reader, status);
    }
}

unsafe extern "C" fn trampoline_requested_incompatible_qos(
    reader: dds_entity_t,
    status: dds_requested_incompatible_qos_status_t,
    arg: *mut c_void,
) {
    if arg.is_null() {
        return;
    }
    let closures = &*(arg as *const ListenerClosures);
    if let Some(ref cb) = closures.on_requested_incompatible_qos {
        cb(reader, status);
    }
}

// ── ListenerBuilder ─────────────────────────────────────────────────

pub struct ListenerBuilder {
    // Existing
    on_data_available: Option<DataAvailableCb>,
    on_publication_matched: Option<PublicationMatchedCb>,
    on_subscription_matched: Option<SubscriptionMatchedCb>,
    on_liveliness_changed: Option<LivelinessChangedCb>,
    // New
    on_inconsistent_topic: Option<InconsistentTopicCb>,
    on_liveliness_lost: Option<LivelinessLostCb>,
    on_offered_deadline_missed: Option<OfferedDeadlineMissedCb>,
    on_offered_incompatible_qos: Option<OfferedIncompatibleQosCb>,
    on_data_on_readers: Option<DataOnReadersCb>,
    on_sample_lost: Option<SampleLostCb>,
    on_sample_rejected: Option<SampleRejectedCb>,
    on_requested_deadline_missed: Option<RequestedDeadlineMissedCb>,
    on_requested_incompatible_qos: Option<RequestedIncompatibleQosCb>,
}

impl ListenerBuilder {
    pub fn new() -> Self {
        ListenerBuilder {
            on_data_available: None,
            on_publication_matched: None,
            on_subscription_matched: None,
            on_liveliness_changed: None,
            on_inconsistent_topic: None,
            on_liveliness_lost: None,
            on_offered_deadline_missed: None,
            on_offered_incompatible_qos: None,
            on_data_on_readers: None,
            on_sample_lost: None,
            on_sample_rejected: None,
            on_requested_deadline_missed: None,
            on_requested_incompatible_qos: None,
        }
    }

    // ── Existing builder methods ────────────────────────────────────

    pub fn on_data_available(mut self, cb: impl Fn(i32) + Send + Sync + 'static) -> Self {
        self.on_data_available = Some(Arc::new(cb));
        self
    }

    pub fn on_publication_matched(
        mut self,
        cb: impl Fn(i32, dds_publication_matched_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_publication_matched = Some(Arc::new(cb));
        self
    }

    pub fn on_subscription_matched(
        mut self,
        cb: impl Fn(i32, dds_subscription_matched_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_subscription_matched = Some(Arc::new(cb));
        self
    }

    pub fn on_liveliness_changed(
        mut self,
        cb: impl Fn(i32, dds_liveliness_changed_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_liveliness_changed = Some(Arc::new(cb));
        self
    }

    // ── New builder methods ─────────────────────────────────────────

    pub fn on_inconsistent_topic(
        mut self,
        cb: impl Fn(i32, dds_inconsistent_topic_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_inconsistent_topic = Some(Arc::new(cb));
        self
    }

    pub fn on_liveliness_lost(
        mut self,
        cb: impl Fn(i32, dds_liveliness_lost_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_liveliness_lost = Some(Arc::new(cb));
        self
    }

    pub fn on_offered_deadline_missed(
        mut self,
        cb: impl Fn(i32, dds_offered_deadline_missed_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_offered_deadline_missed = Some(Arc::new(cb));
        self
    }

    pub fn on_offered_incompatible_qos(
        mut self,
        cb: impl Fn(i32, dds_offered_incompatible_qos_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_offered_incompatible_qos = Some(Arc::new(cb));
        self
    }

    pub fn on_data_on_readers(mut self, cb: impl Fn(i32) + Send + Sync + 'static) -> Self {
        self.on_data_on_readers = Some(Arc::new(cb));
        self
    }

    pub fn on_sample_lost(
        mut self,
        cb: impl Fn(i32, dds_sample_lost_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_sample_lost = Some(Arc::new(cb));
        self
    }

    pub fn on_sample_rejected(
        mut self,
        cb: impl Fn(i32, dds_sample_rejected_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_sample_rejected = Some(Arc::new(cb));
        self
    }

    pub fn on_requested_deadline_missed(
        mut self,
        cb: impl Fn(i32, dds_requested_deadline_missed_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_requested_deadline_missed = Some(Arc::new(cb));
        self
    }

    pub fn on_requested_incompatible_qos(
        mut self,
        cb: impl Fn(i32, dds_requested_incompatible_qos_status_t) + Send + Sync + 'static,
    ) -> Self {
        self.on_requested_incompatible_qos = Some(Arc::new(cb));
        self
    }

    // ── Build ───────────────────────────────────────────────────────

    pub fn build(self) -> DdsResult<Listener> {
        let closures = Box::new(ListenerClosures {
            on_data_available: self.on_data_available,
            on_publication_matched: self.on_publication_matched,
            on_subscription_matched: self.on_subscription_matched,
            on_liveliness_changed: self.on_liveliness_changed,
            on_inconsistent_topic: self.on_inconsistent_topic,
            on_liveliness_lost: self.on_liveliness_lost,
            on_offered_deadline_missed: self.on_offered_deadline_missed,
            on_offered_incompatible_qos: self.on_offered_incompatible_qos,
            on_data_on_readers: self.on_data_on_readers,
            on_sample_lost: self.on_sample_lost,
            on_sample_rejected: self.on_sample_rejected,
            on_requested_deadline_missed: self.on_requested_deadline_missed,
            on_requested_incompatible_qos: self.on_requested_incompatible_qos,
        });

        let arg_ptr = &*closures as *const ListenerClosures as *mut c_void;
        let ptr = unsafe { dds_create_listener(arg_ptr) };
        if ptr.is_null() {
            return Err(crate::DdsError::OutOfResources);
        }

        unsafe {
            // Existing
            if closures.on_data_available.is_some() {
                dds_lset_data_available(ptr, Some(trampoline_data_available));
            }
            if closures.on_publication_matched.is_some() {
                dds_lset_publication_matched(ptr, Some(trampoline_publication_matched));
            }
            if closures.on_subscription_matched.is_some() {
                dds_lset_subscription_matched(ptr, Some(trampoline_subscription_matched));
            }
            if closures.on_liveliness_changed.is_some() {
                dds_lset_liveliness_changed(ptr, Some(trampoline_liveliness_changed));
            }
            // New
            if closures.on_inconsistent_topic.is_some() {
                dds_lset_inconsistent_topic(ptr, Some(trampoline_inconsistent_topic));
            }
            if closures.on_liveliness_lost.is_some() {
                dds_lset_liveliness_lost(ptr, Some(trampoline_liveliness_lost));
            }
            if closures.on_offered_deadline_missed.is_some() {
                dds_lset_offered_deadline_missed(ptr, Some(trampoline_offered_deadline_missed));
            }
            if closures.on_offered_incompatible_qos.is_some() {
                dds_lset_offered_incompatible_qos(ptr, Some(trampoline_offered_incompatible_qos));
            }
            if closures.on_data_on_readers.is_some() {
                dds_lset_data_on_readers(ptr, Some(trampoline_data_on_readers));
            }
            if closures.on_sample_lost.is_some() {
                dds_lset_sample_lost(ptr, Some(trampoline_sample_lost));
            }
            if closures.on_sample_rejected.is_some() {
                dds_lset_sample_rejected(ptr, Some(trampoline_sample_rejected));
            }
            if closures.on_requested_deadline_missed.is_some() {
                dds_lset_requested_deadline_missed(ptr, Some(trampoline_requested_deadline_missed));
            }
            if closures.on_requested_incompatible_qos.is_some() {
                dds_lset_requested_incompatible_qos(
                    ptr,
                    Some(trampoline_requested_incompatible_qos),
                );
            }
        }

        Ok(Listener {
            ptr,
            _closures: closures,
        })
    }
}

impl Default for ListenerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Listener {
    pub fn builder() -> ListenerBuilder {
        ListenerBuilder::new()
    }

    pub fn as_ptr(&self) -> *mut dds_listener_t {
        self.ptr
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        unsafe {
            dds_delete_listener(self.ptr);
        }
    }
}
