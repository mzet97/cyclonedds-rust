use crate::DdsResult;
use cyclonedds_rust_sys::*;
use std::ffi::{CStr, CString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durability {
    Volatile,
    TransientLocal,
    Transient,
    Persistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reliability {
    BestEffort,
    Reliable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum History {
    KeepLast(i32),
    KeepAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ownership {
    Shared,
    Exclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveliness {
    Automatic,
    ManualByParticipant,
    ManualByTopic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationOrder {
    ByReceptionTimestamp,
    BySourceTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationAccessScope {
    Instance,
    Topic,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IgnoreLocalKind {
    None,
    Participant,
    Process,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentationPolicy {
    pub access_scope: PresentationAccessScope,
    pub coherent_access: bool,
    pub ordered_access: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurabilityServicePolicy {
    pub history: History,
    pub max_samples: i32,
    pub max_instances: i32,
    pub max_samples_per_instance: i32,
    pub service_cleanup_delay: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReaderDataLifecyclePolicy {
    pub autopurge_nowriter_delay: i64,
    pub autopurge_disposed_delay: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeConsistency {
    DisallowTypeCoercion,
    AllowTypeCoercion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeConsistencyPolicy {
    pub kind: TypeConsistency,
    pub ignore_sequence_bounds: bool,
    pub ignore_string_bounds: bool,
    pub ignore_member_names: bool,
    pub prevent_type_widening: bool,
    pub force_type_validation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataRepresentation {
    Xcdr1,
    Xml,
    Xcdr2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceLimitsPolicy {
    pub max_samples: i32,
    pub max_instances: i32,
    pub max_samples_per_instance: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeBasedFilterPolicy {
    pub minimum_separation: i64,
}

pub struct QosBuilder {
    reliability: Option<(Reliability, i64)>,
    durability: Option<Durability>,
    history: Option<History>,
    deadline: Option<i64>,
    lifespan: Option<i64>,
    latency_budget: Option<i64>,
    ownership: Option<Ownership>,
    ownership_strength: Option<i32>,
    liveliness: Option<(Liveliness, i64)>,
    destination_order: Option<DestinationOrder>,
    writer_data_lifecycle: Option<bool>,
    transport_priority: Option<i32>,
    partition: Option<String>,
    entity_name: Option<String>,
    userdata: Option<Vec<u8>>,
    topicdata: Option<Vec<u8>>,
    groupdata: Option<Vec<u8>>,
    presentation: Option<(PresentationAccessScope, bool, bool)>,
    durability_service: Option<(History, i32, i32, i32)>,
    ignore_local: Option<IgnoreLocalKind>,
    writer_batching: Option<bool>,
    reader_data_lifecycle: Option<(i64, i64)>,
    type_consistency: Option<TypeConsistencyPolicy>,
    data_representation: Option<Vec<DataRepresentation>>,
    resource_limits: Option<ResourceLimitsPolicy>,
    time_based_filter: Option<TimeBasedFilterPolicy>,
    psmx_instances: Option<Vec<String>>,
    properties: Vec<(String, String, bool)>,
    binary_properties: Vec<(String, Vec<u8>, bool)>,
}

impl QosBuilder {
    pub fn new() -> Self {
        QosBuilder {
            reliability: None,
            durability: None,
            history: None,
            deadline: None,
            lifespan: None,
            latency_budget: None,
            ownership: None,
            ownership_strength: None,
            liveliness: None,
            destination_order: None,
            writer_data_lifecycle: None,
            transport_priority: None,
            partition: None,
            entity_name: None,
            userdata: None,
            topicdata: None,
            groupdata: None,
            presentation: None,
            durability_service: None,
            ignore_local: None,
            writer_batching: None,
            reader_data_lifecycle: None,
            type_consistency: None,
            data_representation: None,
            resource_limits: None,
            time_based_filter: None,
            psmx_instances: None,
            properties: Vec::new(),
            binary_properties: Vec::new(),
        }
    }

    pub fn reliability(mut self, kind: Reliability, max_blocking_time: i64) -> Self {
        self.reliability = Some((kind, max_blocking_time));
        self
    }

    pub fn reliable(self) -> Self {
        self.reliability(Reliability::Reliable, 1_000_000_000)
    }

    pub fn best_effort(self) -> Self {
        self.reliability(Reliability::BestEffort, 0)
    }

    pub fn durability(mut self, kind: Durability) -> Self {
        self.durability = Some(kind);
        self
    }

    pub fn transient_local(self) -> Self {
        self.durability(Durability::TransientLocal)
    }

    pub fn volatile(self) -> Self {
        self.durability(Durability::Volatile)
    }

    pub fn history(mut self, kind: History) -> Self {
        self.history = Some(kind);
        self
    }

    pub fn keep_last(self, depth: i32) -> Self {
        self.history(History::KeepLast(depth))
    }

    pub fn keep_all(self) -> Self {
        self.history(History::KeepAll)
    }

    pub fn deadline(mut self, deadline: i64) -> Self {
        self.deadline = Some(deadline);
        self
    }

    pub fn lifespan(mut self, lifespan: i64) -> Self {
        self.lifespan = Some(lifespan);
        self
    }

    pub fn latency_budget(mut self, duration: i64) -> Self {
        self.latency_budget = Some(duration);
        self
    }

    pub fn ownership(mut self, kind: Ownership) -> Self {
        self.ownership = Some(kind);
        self
    }

    pub fn ownership_strength(mut self, value: i32) -> Self {
        self.ownership_strength = Some(value);
        self
    }

    pub fn liveliness(mut self, kind: Liveliness, lease_duration: i64) -> Self {
        self.liveliness = Some((kind, lease_duration));
        self
    }

    pub fn destination_order(mut self, kind: DestinationOrder) -> Self {
        self.destination_order = Some(kind);
        self
    }

    pub fn writer_data_lifecycle(mut self, autodispose: bool) -> Self {
        self.writer_data_lifecycle = Some(autodispose);
        self
    }

    pub fn transport_priority(mut self, value: i32) -> Self {
        self.transport_priority = Some(value);
        self
    }

    pub fn partition(mut self, name: impl Into<String>) -> Self {
        self.partition = Some(name.into());
        self
    }

    pub fn entity_name(mut self, name: impl Into<String>) -> Self {
        self.entity_name = Some(name.into());
        self
    }

    pub fn userdata(mut self, data: Vec<u8>) -> Self {
        self.userdata = Some(data);
        self
    }

    pub fn topicdata(mut self, data: Vec<u8>) -> Self {
        self.topicdata = Some(data);
        self
    }

    pub fn groupdata(mut self, data: Vec<u8>) -> Self {
        self.groupdata = Some(data);
        self
    }

    pub fn presentation(
        mut self,
        access_scope: PresentationAccessScope,
        coherent_access: bool,
        ordered_access: bool,
    ) -> Self {
        self.presentation = Some((access_scope, coherent_access, ordered_access));
        self
    }

    pub fn durability_service(
        mut self,
        history: History,
        max_samples: i32,
        max_instances: i32,
        max_samples_per_instance: i32,
    ) -> Self {
        self.durability_service = Some((
            history,
            max_samples,
            max_instances,
            max_samples_per_instance,
        ));
        self
    }

    pub fn ignore_local(mut self, kind: IgnoreLocalKind) -> Self {
        self.ignore_local = Some(kind);
        self
    }

    pub fn writer_batching(mut self, enabled: bool) -> Self {
        self.writer_batching = Some(enabled);
        self
    }

    pub fn reader_data_lifecycle(
        mut self,
        autopurge_nowriter_delay: i64,
        autopurge_disposed_delay: i64,
    ) -> Self {
        self.reader_data_lifecycle = Some((autopurge_nowriter_delay, autopurge_disposed_delay));
        self
    }

    pub fn type_consistency(mut self, policy: TypeConsistencyPolicy) -> Self {
        self.type_consistency = Some(policy);
        self
    }

    pub fn data_representation(mut self, values: Vec<DataRepresentation>) -> Self {
        self.data_representation = Some(values);
        self
    }

    pub fn resource_limits(
        mut self,
        max_samples: i32,
        max_instances: i32,
        max_samples_per_instance: i32,
    ) -> Self {
        self.resource_limits = Some(ResourceLimitsPolicy {
            max_samples,
            max_instances,
            max_samples_per_instance,
        });
        self
    }

    pub fn time_based_filter(mut self, minimum_separation: i64) -> Self {
        self.time_based_filter = Some(TimeBasedFilterPolicy { minimum_separation });
        self
    }

    pub fn psmx_instances(mut self, values: Vec<String>) -> Self {
        self.psmx_instances = Some(values);
        self
    }

    pub fn property(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.push((name.into(), value.into(), false));
        self
    }

    pub fn property_propagate(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        propagate: bool,
    ) -> Self {
        self.properties.push((name.into(), value.into(), propagate));
        self
    }

    pub fn binary_property(mut self, name: impl Into<String>, value: Vec<u8>) -> Self {
        self.binary_properties.push((name.into(), value, false));
        self
    }

    pub fn binary_property_propagate(
        mut self,
        name: impl Into<String>,
        value: Vec<u8>,
        propagate: bool,
    ) -> Self {
        self.binary_properties.push((name.into(), value, propagate));
        self
    }

    pub fn build(self) -> DdsResult<Qos> {
        let qos = Qos::create()?;
        unsafe {
            if let Some((kind, max_block)) = self.reliability {
                let c_kind = match kind {
                    Reliability::BestEffort => {
                        dds_reliability_kind_DDS_RELIABILITY_BEST_EFFORT as dds_reliability_kind_t
                    }
                    Reliability::Reliable => {
                        dds_reliability_kind_DDS_RELIABILITY_RELIABLE as dds_reliability_kind_t
                    }
                };
                dds_qset_reliability(qos.ptr, c_kind, max_block);
            }

            if let Some(kind) = self.durability {
                let c_kind = match kind {
                    Durability::Volatile => {
                        dds_durability_kind_DDS_DURABILITY_VOLATILE as dds_durability_kind_t
                    }
                    Durability::TransientLocal => {
                        dds_durability_kind_DDS_DURABILITY_TRANSIENT_LOCAL as dds_durability_kind_t
                    }
                    Durability::Transient => {
                        dds_durability_kind_DDS_DURABILITY_TRANSIENT as dds_durability_kind_t
                    }
                    Durability::Persistent => {
                        dds_durability_kind_DDS_DURABILITY_PERSISTENT as dds_durability_kind_t
                    }
                };
                dds_qset_durability(qos.ptr, c_kind);
            }

            if let Some(hist) = self.history {
                match hist {
                    History::KeepLast(depth) => {
                        dds_qset_history(
                            qos.ptr,
                            dds_history_kind_DDS_HISTORY_KEEP_LAST as dds_history_kind_t,
                            depth,
                        );
                    }
                    History::KeepAll => {
                        dds_qset_history(
                            qos.ptr,
                            dds_history_kind_DDS_HISTORY_KEEP_ALL as dds_history_kind_t,
                            0,
                        );
                    }
                }
            }

            if let Some(deadline) = self.deadline {
                dds_qset_deadline(qos.ptr, deadline);
            }

            if let Some(lifespan) = self.lifespan {
                dds_qset_lifespan(qos.ptr, lifespan);
            }

            if let Some(dur) = self.latency_budget {
                dds_qset_latency_budget(qos.ptr, dur);
            }

            if let Some(kind) = self.ownership {
                let c_kind = match kind {
                    Ownership::Shared => {
                        dds_ownership_kind_DDS_OWNERSHIP_SHARED as dds_ownership_kind_t
                    }
                    Ownership::Exclusive => {
                        dds_ownership_kind_DDS_OWNERSHIP_EXCLUSIVE as dds_ownership_kind_t
                    }
                };
                dds_qset_ownership(qos.ptr, c_kind);
            }

            if let Some(strength) = self.ownership_strength {
                dds_qset_ownership_strength(qos.ptr, strength);
            }

            if let Some((kind, lease)) = self.liveliness {
                let c_kind = match kind {
                    Liveliness::Automatic => {
                        dds_liveliness_kind_DDS_LIVELINESS_AUTOMATIC as dds_liveliness_kind_t
                    }
                    Liveliness::ManualByParticipant => {
                        dds_liveliness_kind_DDS_LIVELINESS_MANUAL_BY_PARTICIPANT
                            as dds_liveliness_kind_t
                    }
                    Liveliness::ManualByTopic => {
                        dds_liveliness_kind_DDS_LIVELINESS_MANUAL_BY_TOPIC as dds_liveliness_kind_t
                    }
                };
                dds_qset_liveliness(qos.ptr, c_kind, lease);
            }

            if let Some(kind) = self.destination_order {
                let c_kind = match kind {
                    DestinationOrder::ByReceptionTimestamp => {
                        dds_destination_order_kind_DDS_DESTINATIONORDER_BY_RECEPTION_TIMESTAMP
                            as dds_destination_order_kind_t
                    }
                    DestinationOrder::BySourceTimestamp => {
                        dds_destination_order_kind_DDS_DESTINATIONORDER_BY_SOURCE_TIMESTAMP
                            as dds_destination_order_kind_t
                    }
                };
                dds_qset_destination_order(qos.ptr, c_kind);
            }

            if let Some(autodispose) = self.writer_data_lifecycle {
                dds_qset_writer_data_lifecycle(qos.ptr, autodispose);
            }

            if let Some(prio) = self.transport_priority {
                dds_qset_transport_priority(qos.ptr, prio);
            }

            if let Some(ref name) = self.partition {
                let c_name = CString::new(name.as_str()).unwrap();
                dds_qset_partition1(qos.ptr, c_name.as_ptr());
            }

            if let Some(ref name) = self.entity_name {
                let c_name = CString::new(name.as_str()).unwrap();
                dds_qset_entity_name(qos.ptr, c_name.as_ptr());
            }

            if let Some(ref data) = self.userdata {
                dds_qset_userdata(qos.ptr, data.as_ptr() as *const _, data.len());
            }

            if let Some(ref data) = self.topicdata {
                dds_qset_topicdata(qos.ptr, data.as_ptr() as *const _, data.len());
            }

            if let Some(ref data) = self.groupdata {
                dds_qset_groupdata(qos.ptr, data.as_ptr() as *const _, data.len());
            }

            if let Some((scope, coherent, ordered)) = self.presentation {
                let c_scope = match scope {
                    PresentationAccessScope::Instance => {
                        dds_presentation_access_scope_kind_DDS_PRESENTATION_INSTANCE
                            as dds_presentation_access_scope_kind_t
                    }
                    PresentationAccessScope::Topic => {
                        dds_presentation_access_scope_kind_DDS_PRESENTATION_TOPIC
                            as dds_presentation_access_scope_kind_t
                    }
                    PresentationAccessScope::Group => {
                        dds_presentation_access_scope_kind_DDS_PRESENTATION_GROUP
                            as dds_presentation_access_scope_kind_t
                    }
                };
                dds_qset_presentation(qos.ptr, c_scope, coherent, ordered);
            }

            if let Some((hist, max_samples, max_instances, max_per_inst)) = self.durability_service
            {
                let (c_kind, depth) = match hist {
                    History::KeepLast(d) => (
                        dds_history_kind_DDS_HISTORY_KEEP_LAST as dds_history_kind_t,
                        d,
                    ),
                    History::KeepAll => (
                        dds_history_kind_DDS_HISTORY_KEEP_ALL as dds_history_kind_t,
                        0,
                    ),
                };
                dds_qset_durability_service(
                    qos.ptr,
                    0,
                    c_kind,
                    depth,
                    max_samples,
                    max_instances,
                    max_per_inst,
                );
            }

            if let Some(kind) = self.ignore_local {
                let c_kind = match kind {
                    IgnoreLocalKind::None => {
                        dds_ignorelocal_kind_DDS_IGNORELOCAL_NONE as dds_ignorelocal_kind_t
                    }
                    IgnoreLocalKind::Participant => {
                        dds_ignorelocal_kind_DDS_IGNORELOCAL_PARTICIPANT as dds_ignorelocal_kind_t
                    }
                    IgnoreLocalKind::Process => {
                        dds_ignorelocal_kind_DDS_IGNORELOCAL_PROCESS as dds_ignorelocal_kind_t
                    }
                };
                dds_qset_ignorelocal(qos.ptr, c_kind);
            }

            if let Some(enabled) = self.writer_batching {
                dds_qset_writer_batching(qos.ptr, enabled);
            }

            if let Some((autopurge_nowriter, autopurge_disposed)) = self.reader_data_lifecycle {
                dds_qset_reader_data_lifecycle(qos.ptr, autopurge_nowriter, autopurge_disposed);
            }

            if let Some(policy) = self.type_consistency {
                let kind = match policy.kind {
                    TypeConsistency::DisallowTypeCoercion => {
                        dds_type_consistency_kind_DDS_TYPE_CONSISTENCY_DISALLOW_TYPE_COERCION
                    }
                    TypeConsistency::AllowTypeCoercion => {
                        dds_type_consistency_kind_DDS_TYPE_CONSISTENCY_ALLOW_TYPE_COERCION
                    }
                } as dds_type_consistency_kind_t;
                dds_qset_type_consistency(
                    qos.ptr,
                    kind,
                    policy.ignore_sequence_bounds,
                    policy.ignore_string_bounds,
                    policy.ignore_member_names,
                    policy.prevent_type_widening,
                    policy.force_type_validation,
                );
            }

            if let Some(values) = self.data_representation {
                let raw: Vec<dds_data_representation_id_t> = values
                    .into_iter()
                    .map(|value| match value {
                        DataRepresentation::Xcdr1 => {
                            DDS_DATA_REPRESENTATION_XCDR1 as dds_data_representation_id_t
                        }
                        DataRepresentation::Xml => {
                            DDS_DATA_REPRESENTATION_XML as dds_data_representation_id_t
                        }
                        DataRepresentation::Xcdr2 => {
                            DDS_DATA_REPRESENTATION_XCDR2 as dds_data_representation_id_t
                        }
                    })
                    .collect();
                dds_qset_data_representation(qos.ptr, raw.len() as u32, raw.as_ptr());
            }

            if let Some(policy) = self.resource_limits {
                dds_qset_resource_limits(
                    qos.ptr,
                    policy.max_samples,
                    policy.max_instances,
                    policy.max_samples_per_instance,
                );
            }

            if let Some(policy) = self.time_based_filter {
                dds_qset_time_based_filter(qos.ptr, policy.minimum_separation);
            }

            if let Some(values) = self.psmx_instances {
                let cstrings: Vec<CString> = values
                    .iter()
                    .map(|v| CString::new(v.as_str()).unwrap())
                    .collect();
                let mut ptrs: Vec<*const ::std::ffi::c_char> =
                    cstrings.iter().map(|v| v.as_ptr()).collect();
                dds_qset_psmx_instances(qos.ptr, ptrs.len() as u32, ptrs.as_mut_ptr());
            }

            for (name, value, propagate) in self.properties {
                let c_name = CString::new(name).unwrap();
                let c_value = CString::new(value).unwrap();
                if propagate {
                    dds_qset_prop_propagate(qos.ptr, c_name.as_ptr(), c_value.as_ptr(), true);
                } else {
                    dds_qset_prop(qos.ptr, c_name.as_ptr(), c_value.as_ptr());
                }
            }

            for (name, value, propagate) in self.binary_properties {
                let c_name = CString::new(name).unwrap();
                if propagate {
                    dds_qset_bprop_propagate(
                        qos.ptr,
                        c_name.as_ptr(),
                        value.as_ptr().cast(),
                        value.len(),
                        true,
                    );
                } else {
                    dds_qset_bprop(qos.ptr, c_name.as_ptr(), value.as_ptr().cast(), value.len());
                }
            }
        }
        Ok(qos)
    }
}

impl Default for QosBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Qos {
    ptr: *mut dds_qos_t,
}

impl Qos {
    pub fn create() -> DdsResult<Self> {
        let ptr = unsafe { dds_create_qos() };
        if ptr.is_null() {
            Err(crate::DdsError::OutOfResources)
        } else {
            Ok(Qos { ptr })
        }
    }

    pub fn builder() -> QosBuilder {
        QosBuilder::new()
    }

    pub fn as_ptr(&self) -> *const dds_qos_t {
        self.ptr
    }

    pub(crate) fn as_mut_ptr(&mut self) -> *mut dds_qos_t {
        self.ptr
    }

    pub(crate) fn from_raw_clone(ptr: *const dds_qos_t) -> crate::DdsResult<Option<Self>> {
        if ptr.is_null() {
            return Ok(None);
        }
        let dst = unsafe { dds_create_qos() };
        if dst.is_null() {
            return Err(crate::DdsError::OutOfResources);
        }
        unsafe {
            dds_copy_qos(dst, ptr);
        }
        Ok(Some(Qos { ptr: dst }))
    }

    pub fn equals(&self, other: &Qos) -> bool {
        unsafe { dds_qos_equal(self.ptr, other.ptr) }
    }

    pub fn set_property(&mut self, name: &str, value: &str) -> DdsResult<()> {
        let c_name = CString::new(name)
            .map_err(|_| crate::DdsError::BadParameter("property name contains null".into()))?;
        let c_value = CString::new(value)
            .map_err(|_| crate::DdsError::BadParameter("property value contains null".into()))?;
        unsafe { dds_qset_prop(self.ptr, c_name.as_ptr(), c_value.as_ptr()) }
        Ok(())
    }

    pub fn set_property_propagate(
        &mut self,
        name: &str,
        value: &str,
        propagate: bool,
    ) -> DdsResult<()> {
        let c_name = CString::new(name)
            .map_err(|_| crate::DdsError::BadParameter("property name contains null".into()))?;
        let c_value = CString::new(value)
            .map_err(|_| crate::DdsError::BadParameter("property value contains null".into()))?;
        unsafe { dds_qset_prop_propagate(self.ptr, c_name.as_ptr(), c_value.as_ptr(), propagate) }
        Ok(())
    }

    pub fn unset_property(&mut self, name: &str) -> DdsResult<()> {
        let c_name = CString::new(name)
            .map_err(|_| crate::DdsError::BadParameter("property name contains null".into()))?;
        unsafe { dds_qunset_prop(self.ptr, c_name.as_ptr()) }
        Ok(())
    }

    pub fn set_binary_property(&mut self, name: &str, value: &[u8]) -> DdsResult<()> {
        let c_name = CString::new(name).map_err(|_| {
            crate::DdsError::BadParameter("binary property name contains null".into())
        })?;
        unsafe {
            dds_qset_bprop(
                self.ptr,
                c_name.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
            )
        }
        Ok(())
    }

    pub fn set_binary_property_propagate(
        &mut self,
        name: &str,
        value: &[u8],
        propagate: bool,
    ) -> DdsResult<()> {
        let c_name = CString::new(name).map_err(|_| {
            crate::DdsError::BadParameter("binary property name contains null".into())
        })?;
        unsafe {
            dds_qset_bprop_propagate(
                self.ptr,
                c_name.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
                propagate,
            )
        }
        Ok(())
    }

    pub fn unset_binary_property(&mut self, name: &str) -> DdsResult<()> {
        let c_name = CString::new(name).map_err(|_| {
            crate::DdsError::BadParameter("binary property name contains null".into())
        })?;
        unsafe { dds_qunset_bprop(self.ptr, c_name.as_ptr()) }
        Ok(())
    }

    pub fn reliability(&self) -> DdsResult<Option<(Reliability, i64)>> {
        unsafe {
            let mut kind = 0;
            let mut max_blocking_time = 0;
            if !dds_qget_reliability(self.ptr, &mut kind, &mut max_blocking_time) {
                return Ok(None);
            }
            Ok(Some((
                match kind {
                    x if x
                        == dds_reliability_kind_DDS_RELIABILITY_BEST_EFFORT
                            as dds_reliability_kind_t =>
                    {
                        Reliability::BestEffort
                    }
                    x if x
                        == dds_reliability_kind_DDS_RELIABILITY_RELIABLE
                            as dds_reliability_kind_t =>
                    {
                        Reliability::Reliable
                    }
                    _ => {
                        return Err(crate::DdsError::Other(format!(
                            "unknown reliability kind: {kind}"
                        )))
                    }
                },
                max_blocking_time,
            )))
        }
    }

    pub fn durability(&self) -> DdsResult<Option<Durability>> {
        unsafe {
            let mut kind = 0;
            if !dds_qget_durability(self.ptr, &mut kind) {
                return Ok(None);
            }
            Ok(Some(match kind {
                x if x == dds_durability_kind_DDS_DURABILITY_VOLATILE as dds_durability_kind_t => {
                    Durability::Volatile
                }
                x if x
                    == dds_durability_kind_DDS_DURABILITY_TRANSIENT_LOCAL
                        as dds_durability_kind_t =>
                {
                    Durability::TransientLocal
                }
                x if x == dds_durability_kind_DDS_DURABILITY_TRANSIENT as dds_durability_kind_t => {
                    Durability::Transient
                }
                x if x
                    == dds_durability_kind_DDS_DURABILITY_PERSISTENT as dds_durability_kind_t =>
                {
                    Durability::Persistent
                }
                _ => {
                    return Err(crate::DdsError::Other(format!(
                        "unknown durability kind: {kind}"
                    )))
                }
            }))
        }
    }

    pub fn history(&self) -> DdsResult<Option<History>> {
        unsafe {
            let mut kind = 0;
            let mut depth = 0;
            if !dds_qget_history(self.ptr, &mut kind, &mut depth) {
                return Ok(None);
            }
            Ok(Some(match kind {
                x if x == dds_history_kind_DDS_HISTORY_KEEP_LAST as dds_history_kind_t => {
                    History::KeepLast(depth)
                }
                x if x == dds_history_kind_DDS_HISTORY_KEEP_ALL as dds_history_kind_t => {
                    History::KeepAll
                }
                _ => {
                    return Err(crate::DdsError::Other(format!(
                        "unknown history kind: {kind}"
                    )))
                }
            }))
        }
    }

    pub fn deadline(&self) -> DdsResult<Option<i64>> {
        unsafe {
            let mut value = 0;
            Ok(if dds_qget_deadline(self.ptr, &mut value) {
                Some(value)
            } else {
                None
            })
        }
    }

    pub fn lifespan(&self) -> DdsResult<Option<i64>> {
        unsafe {
            let mut value = 0;
            Ok(if dds_qget_lifespan(self.ptr, &mut value) {
                Some(value)
            } else {
                None
            })
        }
    }

    pub fn latency_budget(&self) -> DdsResult<Option<i64>> {
        unsafe {
            let mut value = 0;
            Ok(if dds_qget_latency_budget(self.ptr, &mut value) {
                Some(value)
            } else {
                None
            })
        }
    }

    pub fn ownership(&self) -> DdsResult<Option<Ownership>> {
        unsafe {
            let mut kind = 0;
            if !dds_qget_ownership(self.ptr, &mut kind) {
                return Ok(None);
            }
            Ok(Some(match kind {
                x if x == dds_ownership_kind_DDS_OWNERSHIP_SHARED as dds_ownership_kind_t => {
                    Ownership::Shared
                }
                x if x == dds_ownership_kind_DDS_OWNERSHIP_EXCLUSIVE as dds_ownership_kind_t => {
                    Ownership::Exclusive
                }
                _ => {
                    return Err(crate::DdsError::Other(format!(
                        "unknown ownership kind: {kind}"
                    )))
                }
            }))
        }
    }

    pub fn ownership_strength(&self) -> DdsResult<Option<i32>> {
        unsafe {
            let mut value = 0;
            Ok(if dds_qget_ownership_strength(self.ptr, &mut value) {
                Some(value)
            } else {
                None
            })
        }
    }

    pub fn liveliness(&self) -> DdsResult<Option<(Liveliness, i64)>> {
        unsafe {
            let mut kind = 0;
            let mut lease = 0;
            if !dds_qget_liveliness(self.ptr, &mut kind, &mut lease) {
                return Ok(None);
            }
            Ok(Some((
                match kind {
                    x if x
                        == dds_liveliness_kind_DDS_LIVELINESS_AUTOMATIC
                            as dds_liveliness_kind_t =>
                    {
                        Liveliness::Automatic
                    }
                    x if x
                        == dds_liveliness_kind_DDS_LIVELINESS_MANUAL_BY_PARTICIPANT
                            as dds_liveliness_kind_t =>
                    {
                        Liveliness::ManualByParticipant
                    }
                    x if x
                        == dds_liveliness_kind_DDS_LIVELINESS_MANUAL_BY_TOPIC
                            as dds_liveliness_kind_t =>
                    {
                        Liveliness::ManualByTopic
                    }
                    _ => {
                        return Err(crate::DdsError::Other(format!(
                            "unknown liveliness kind: {kind}"
                        )))
                    }
                },
                lease,
            )))
        }
    }

    pub fn destination_order(&self) -> DdsResult<Option<DestinationOrder>> {
        unsafe {
            let mut kind = 0;
            if !dds_qget_destination_order(self.ptr, &mut kind) {
                return Ok(None);
            }
            Ok(Some(match kind {
                x if x
                    == dds_destination_order_kind_DDS_DESTINATIONORDER_BY_RECEPTION_TIMESTAMP
                        as dds_destination_order_kind_t =>
                {
                    DestinationOrder::ByReceptionTimestamp
                }
                x if x
                    == dds_destination_order_kind_DDS_DESTINATIONORDER_BY_SOURCE_TIMESTAMP
                        as dds_destination_order_kind_t =>
                {
                    DestinationOrder::BySourceTimestamp
                }
                _ => {
                    return Err(crate::DdsError::Other(format!(
                        "unknown destination order kind: {kind}"
                    )))
                }
            }))
        }
    }

    pub fn writer_data_lifecycle(&self) -> DdsResult<Option<bool>> {
        unsafe {
            let mut value = false;
            Ok(if dds_qget_writer_data_lifecycle(self.ptr, &mut value) {
                Some(value)
            } else {
                None
            })
        }
    }

    pub fn transport_priority(&self) -> DdsResult<Option<i32>> {
        unsafe {
            let mut value = 0;
            Ok(if dds_qget_transport_priority(self.ptr, &mut value) {
                Some(value)
            } else {
                None
            })
        }
    }

    pub fn userdata(&self) -> DdsResult<Option<Vec<u8>>> {
        unsafe {
            let mut ptr = std::ptr::null_mut();
            let mut size = 0usize;
            if !dds_qget_userdata(self.ptr, &mut ptr, &mut size) {
                return Ok(None);
            }
            if ptr.is_null() || size == 0 {
                return Ok(Some(Vec::new()));
            }
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size).to_vec();
            dds_free(ptr);
            Ok(Some(bytes))
        }
    }

    pub fn topicdata(&self) -> DdsResult<Option<Vec<u8>>> {
        unsafe {
            let mut ptr = std::ptr::null_mut();
            let mut size = 0usize;
            if !dds_qget_topicdata(self.ptr, &mut ptr, &mut size) {
                return Ok(None);
            }
            if ptr.is_null() || size == 0 {
                return Ok(Some(Vec::new()));
            }
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size).to_vec();
            dds_free(ptr);
            Ok(Some(bytes))
        }
    }

    pub fn groupdata(&self) -> DdsResult<Option<Vec<u8>>> {
        unsafe {
            let mut ptr = std::ptr::null_mut();
            let mut size = 0usize;
            if !dds_qget_groupdata(self.ptr, &mut ptr, &mut size) {
                return Ok(None);
            }
            if ptr.is_null() || size == 0 {
                return Ok(Some(Vec::new()));
            }
            let bytes = std::slice::from_raw_parts(ptr as *const u8, size).to_vec();
            dds_free(ptr);
            Ok(Some(bytes))
        }
    }

    pub fn presentation(&self) -> DdsResult<Option<PresentationPolicy>> {
        unsafe {
            let mut access_scope = 0;
            let mut coherent_access = false;
            let mut ordered_access = false;
            if !dds_qget_presentation(
                self.ptr,
                &mut access_scope,
                &mut coherent_access,
                &mut ordered_access,
            ) {
                return Ok(None);
            }
            Ok(Some(PresentationPolicy {
                access_scope: match access_scope {
                    x if x
                        == dds_presentation_access_scope_kind_DDS_PRESENTATION_INSTANCE
                            as dds_presentation_access_scope_kind_t =>
                    {
                        PresentationAccessScope::Instance
                    }
                    x if x
                        == dds_presentation_access_scope_kind_DDS_PRESENTATION_TOPIC
                            as dds_presentation_access_scope_kind_t =>
                    {
                        PresentationAccessScope::Topic
                    }
                    x if x
                        == dds_presentation_access_scope_kind_DDS_PRESENTATION_GROUP
                            as dds_presentation_access_scope_kind_t =>
                    {
                        PresentationAccessScope::Group
                    }
                    _ => {
                        return Err(crate::DdsError::Other(format!(
                            "unknown presentation access scope: {access_scope}"
                        )))
                    }
                },
                coherent_access,
                ordered_access,
            }))
        }
    }

    pub fn durability_service(&self) -> DdsResult<Option<DurabilityServicePolicy>> {
        unsafe {
            let mut cleanup_delay = 0;
            let mut history_kind = 0;
            let mut history_depth = 0;
            let mut max_samples = 0;
            let mut max_instances = 0;
            let mut max_samples_per_instance = 0;
            if !dds_qget_durability_service(
                self.ptr,
                &mut cleanup_delay,
                &mut history_kind,
                &mut history_depth,
                &mut max_samples,
                &mut max_instances,
                &mut max_samples_per_instance,
            ) {
                return Ok(None);
            }
            Ok(Some(DurabilityServicePolicy {
                history: match history_kind {
                    x if x == dds_history_kind_DDS_HISTORY_KEEP_LAST as dds_history_kind_t => {
                        History::KeepLast(history_depth)
                    }
                    x if x == dds_history_kind_DDS_HISTORY_KEEP_ALL as dds_history_kind_t => {
                        History::KeepAll
                    }
                    _ => {
                        return Err(crate::DdsError::Other(format!(
                            "unknown durability-service history kind: {history_kind}"
                        )))
                    }
                },
                max_samples,
                max_instances,
                max_samples_per_instance,
                service_cleanup_delay: cleanup_delay,
            }))
        }
    }

    pub fn ignore_local(&self) -> DdsResult<Option<IgnoreLocalKind>> {
        unsafe {
            let mut kind = 0;
            if !dds_qget_ignorelocal(self.ptr, &mut kind) {
                return Ok(None);
            }
            Ok(Some(match kind {
                x if x == dds_ignorelocal_kind_DDS_IGNORELOCAL_NONE as dds_ignorelocal_kind_t => {
                    IgnoreLocalKind::None
                }
                x if x
                    == dds_ignorelocal_kind_DDS_IGNORELOCAL_PARTICIPANT
                        as dds_ignorelocal_kind_t =>
                {
                    IgnoreLocalKind::Participant
                }
                x if x
                    == dds_ignorelocal_kind_DDS_IGNORELOCAL_PROCESS as dds_ignorelocal_kind_t =>
                {
                    IgnoreLocalKind::Process
                }
                _ => {
                    return Err(crate::DdsError::Other(format!(
                        "unknown ignore-local kind: {kind}"
                    )))
                }
            }))
        }
    }

    pub fn writer_batching(&self) -> DdsResult<Option<bool>> {
        unsafe {
            let mut value = false;
            Ok(if dds_qget_writer_batching(self.ptr, &mut value) {
                Some(value)
            } else {
                None
            })
        }
    }

    pub fn reader_data_lifecycle(&self) -> DdsResult<Option<ReaderDataLifecyclePolicy>> {
        unsafe {
            let mut nowriter = 0;
            let mut disposed = 0;
            if !dds_qget_reader_data_lifecycle(self.ptr, &mut nowriter, &mut disposed) {
                return Ok(None);
            }
            Ok(Some(ReaderDataLifecyclePolicy {
                autopurge_nowriter_delay: nowriter,
                autopurge_disposed_delay: disposed,
            }))
        }
    }

    pub fn type_consistency(&self) -> DdsResult<Option<TypeConsistencyPolicy>> {
        unsafe {
            let mut kind = 0;
            let mut ignore_sequence_bounds = false;
            let mut ignore_string_bounds = false;
            let mut ignore_member_names = false;
            let mut prevent_type_widening = false;
            let mut force_type_validation = false;
            if !dds_qget_type_consistency(
                self.ptr,
                &mut kind,
                &mut ignore_sequence_bounds,
                &mut ignore_string_bounds,
                &mut ignore_member_names,
                &mut prevent_type_widening,
                &mut force_type_validation,
            ) {
                return Ok(None);
            }
            let kind = match kind {
                x if x
                    == dds_type_consistency_kind_DDS_TYPE_CONSISTENCY_DISALLOW_TYPE_COERCION
                        as dds_type_consistency_kind_t =>
                {
                    TypeConsistency::DisallowTypeCoercion
                }
                x if x
                    == dds_type_consistency_kind_DDS_TYPE_CONSISTENCY_ALLOW_TYPE_COERCION
                        as dds_type_consistency_kind_t =>
                {
                    TypeConsistency::AllowTypeCoercion
                }
                _ => {
                    return Err(crate::DdsError::Other(format!(
                        "unknown type consistency kind: {kind}"
                    )))
                }
            };
            Ok(Some(TypeConsistencyPolicy {
                kind,
                ignore_sequence_bounds,
                ignore_string_bounds,
                ignore_member_names,
                prevent_type_widening,
                force_type_validation,
            }))
        }
    }

    pub fn data_representation(&self) -> DdsResult<Option<Vec<DataRepresentation>>> {
        unsafe {
            let mut n = 0u32;
            let mut values = std::ptr::null_mut();
            if !dds_qget_data_representation(self.ptr, &mut n, &mut values) {
                return Ok(None);
            }
            if values.is_null() || n == 0 {
                return Ok(Some(Vec::new()));
            }
            let slice = std::slice::from_raw_parts(values, n as usize);
            let result = slice
                .iter()
                .map(|value| match *value as u32 {
                    x if x == DDS_DATA_REPRESENTATION_XCDR1 => Ok(DataRepresentation::Xcdr1),
                    x if x == DDS_DATA_REPRESENTATION_XML => Ok(DataRepresentation::Xml),
                    x if x == DDS_DATA_REPRESENTATION_XCDR2 => Ok(DataRepresentation::Xcdr2),
                    other => Err(crate::DdsError::Other(format!(
                        "unknown data representation id: {other}"
                    ))),
                })
                .collect::<crate::DdsResult<Vec<_>>>()?;
            dds_free(values.cast());
            Ok(Some(result))
        }
    }

    pub fn partition(&self) -> DdsResult<Option<Vec<String>>> {
        unsafe {
            let mut n = 0u32;
            let mut values = std::ptr::null_mut();
            if !dds_qget_partition(self.ptr, &mut n, &mut values) {
                return Ok(None);
            }
            if values.is_null() || n == 0 {
                return Ok(Some(Vec::new()));
            }
            let slice = std::slice::from_raw_parts(values, n as usize);
            let result = slice
                .iter()
                .map(|value| {
                    if value.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(*value).to_string_lossy().into_owned()
                    }
                })
                .collect::<Vec<_>>();
            for value in slice {
                if !value.is_null() {
                    dds_string_free(*value);
                }
            }
            dds_free(values.cast());
            Ok(Some(result))
        }
    }

    pub fn resource_limits(&self) -> DdsResult<Option<ResourceLimitsPolicy>> {
        unsafe {
            let mut max_samples = 0;
            let mut max_instances = 0;
            let mut max_samples_per_instance = 0;
            if !dds_qget_resource_limits(
                self.ptr,
                &mut max_samples,
                &mut max_instances,
                &mut max_samples_per_instance,
            ) {
                return Ok(None);
            }
            Ok(Some(ResourceLimitsPolicy {
                max_samples,
                max_instances,
                max_samples_per_instance,
            }))
        }
    }

    pub fn time_based_filter(&self) -> DdsResult<Option<TimeBasedFilterPolicy>> {
        unsafe {
            let mut minimum_separation = 0;
            if !dds_qget_time_based_filter(self.ptr, &mut minimum_separation) {
                return Ok(None);
            }
            Ok(Some(TimeBasedFilterPolicy { minimum_separation }))
        }
    }

    pub fn psmx_instances(&self) -> DdsResult<Option<Vec<String>>> {
        unsafe {
            let mut n = 0u32;
            let mut values = std::ptr::null_mut();
            if !dds_qget_psmx_instances(self.ptr, &mut n, &mut values) {
                return Ok(None);
            }
            if values.is_null() || n == 0 {
                return Ok(Some(Vec::new()));
            }
            let slice = std::slice::from_raw_parts(values, n as usize);
            let result = slice
                .iter()
                .map(|value| {
                    if value.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(*value).to_string_lossy().into_owned()
                    }
                })
                .collect::<Vec<_>>();
            for value in slice {
                if !value.is_null() {
                    dds_string_free(*value);
                }
            }
            dds_free(values.cast());
            Ok(Some(result))
        }
    }

    pub fn property(&self, name: &str) -> DdsResult<Option<String>> {
        unsafe {
            let c_name = CString::new(name)
                .map_err(|_| crate::DdsError::BadParameter("property name contains null".into()))?;
            let mut value = std::ptr::null_mut();
            if !dds_qget_prop(self.ptr, c_name.as_ptr(), &mut value) {
                return Ok(None);
            }
            if value.is_null() {
                return Ok(Some(String::new()));
            }
            let result = CStr::from_ptr(value).to_string_lossy().into_owned();
            dds_string_free(value);
            Ok(Some(result))
        }
    }

    pub fn property_propagate(&self, name: &str) -> DdsResult<Option<(String, bool)>> {
        unsafe {
            let c_name = CString::new(name)
                .map_err(|_| crate::DdsError::BadParameter("property name contains null".into()))?;
            let mut value = std::ptr::null_mut();
            let mut propagate = false;
            if !dds_qget_prop_propagate(self.ptr, c_name.as_ptr(), &mut value, &mut propagate) {
                return Ok(None);
            }
            if value.is_null() {
                return Ok(Some((String::new(), propagate)));
            }
            let result = CStr::from_ptr(value).to_string_lossy().into_owned();
            dds_string_free(value);
            Ok(Some((result, propagate)))
        }
    }

    pub fn property_names(&self) -> DdsResult<Option<Vec<String>>> {
        unsafe {
            let mut n = 0u32;
            let mut names = std::ptr::null_mut();
            if !dds_qget_propnames(self.ptr, &mut n, &mut names) {
                return Ok(None);
            }
            if names.is_null() || n == 0 {
                return Ok(Some(Vec::new()));
            }
            let slice = std::slice::from_raw_parts(names, n as usize);
            let result = slice
                .iter()
                .map(|name| {
                    if name.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(*name).to_string_lossy().into_owned()
                    }
                })
                .collect::<Vec<_>>();
            for name in slice {
                if !name.is_null() {
                    dds_string_free(*name);
                }
            }
            dds_free(names.cast());
            Ok(Some(result))
        }
    }

    pub fn binary_property(&self, name: &str) -> DdsResult<Option<Vec<u8>>> {
        unsafe {
            let c_name = CString::new(name).map_err(|_| {
                crate::DdsError::BadParameter("binary property name contains null".into())
            })?;
            let mut value = std::ptr::null_mut();
            let mut size = 0usize;
            if !dds_qget_bprop(self.ptr, c_name.as_ptr(), &mut value, &mut size) {
                return Ok(None);
            }
            if value.is_null() || size == 0 {
                return Ok(Some(Vec::new()));
            }
            let result = std::slice::from_raw_parts(value.cast::<u8>(), size).to_vec();
            dds_free(value);
            Ok(Some(result))
        }
    }

    pub fn binary_property_propagate(&self, name: &str) -> DdsResult<Option<(Vec<u8>, bool)>> {
        unsafe {
            let c_name = CString::new(name).map_err(|_| {
                crate::DdsError::BadParameter("binary property name contains null".into())
            })?;
            let mut value = std::ptr::null_mut();
            let mut size = 0usize;
            let mut propagate = false;
            if !dds_qget_bprop_propagate(
                self.ptr,
                c_name.as_ptr(),
                &mut value,
                &mut size,
                &mut propagate,
            ) {
                return Ok(None);
            }
            if value.is_null() || size == 0 {
                return Ok(Some((Vec::new(), propagate)));
            }
            let result = std::slice::from_raw_parts(value.cast::<u8>(), size).to_vec();
            dds_free(value);
            Ok(Some((result, propagate)))
        }
    }

    pub fn binary_property_names(&self) -> DdsResult<Option<Vec<String>>> {
        unsafe {
            let mut n = 0u32;
            let mut names = std::ptr::null_mut();
            if !dds_qget_bpropnames(self.ptr, &mut n, &mut names) {
                return Ok(None);
            }
            if names.is_null() || n == 0 {
                return Ok(Some(Vec::new()));
            }
            let slice = std::slice::from_raw_parts(names, n as usize);
            let result = slice
                .iter()
                .map(|name| {
                    if name.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(*name).to_string_lossy().into_owned()
                    }
                })
                .collect::<Vec<_>>();
            for name in slice {
                if !name.is_null() {
                    dds_string_free(*name);
                }
            }
            dds_free(names.cast());
            Ok(Some(result))
        }
    }

    pub fn entity_name(&self) -> DdsResult<Option<String>> {
        unsafe {
            let mut ptr = std::ptr::null_mut();
            if !dds_qget_entity_name(self.ptr, &mut ptr) {
                return Ok(None);
            }
            if ptr.is_null() {
                return Ok(Some(String::new()));
            }
            let value = CStr::from_ptr(ptr).to_string_lossy().into_owned();
            dds_string_free(ptr);
            Ok(Some(value))
        }
    }
}

impl Clone for Qos {
    fn clone(&self) -> Self {
        let dst = unsafe { dds_create_qos() };
        unsafe {
            dds_copy_qos(dst, self.ptr);
        }
        Qos { ptr: dst }
    }
}

impl Drop for Qos {
    fn drop(&mut self) {
        unsafe {
            dds_delete_qos(self.ptr);
        }
    }
}

impl Default for Qos {
    fn default() -> Self {
        Self::create().expect("failed to create Qos")
    }
}
