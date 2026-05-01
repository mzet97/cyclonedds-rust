//! DDS Status API – typed status structs and status-mask constants.
//!
//! Each DDS entity type can emit specific status events.  This module
//! provides Rust-friendly wrappers around the C `dds_get_*_status`
//! functions and the corresponding bit-mask constants.

use crate::{DdsEntity, DdsResult};
use cyclonedds_rust_sys::*;

// ---------------------------------------------------------------------------
// Status-mask constants  (1 << id, matching dds_status_id values)
// ---------------------------------------------------------------------------

/// Inconsistent topic status bit.
pub const STATUS_INCONSISTENT_TOPIC: u32 = 1u32 << dds_status_id_DDS_INCONSISTENT_TOPIC_STATUS_ID;
/// Offered deadline missed status bit (writer).
pub const STATUS_OFFERED_DEADLINE_MISSED: u32 =
    1u32 << dds_status_id_DDS_OFFERED_DEADLINE_MISSED_STATUS_ID;
/// Requested deadline missed status bit (reader).
pub const STATUS_REQUESTED_DEADLINE_MISSED: u32 =
    1u32 << dds_status_id_DDS_REQUESTED_DEADLINE_MISSED_STATUS_ID;
/// Offered incompatible QoS status bit (writer).
pub const STATUS_OFFERED_INCOMPATIBLE_QOS: u32 =
    1u32 << dds_status_id_DDS_OFFERED_INCOMPATIBLE_QOS_STATUS_ID;
/// Requested incompatible QoS status bit (reader).
pub const STATUS_REQUESTED_INCOMPATIBLE_QOS: u32 =
    1u32 << dds_status_id_DDS_REQUESTED_INCOMPATIBLE_QOS_STATUS_ID;
/// Sample lost status bit (reader).
pub const STATUS_SAMPLE_LOST: u32 = 1u32 << dds_status_id_DDS_SAMPLE_LOST_STATUS_ID;
/// Sample rejected status bit (reader).
pub const STATUS_SAMPLE_REJECTED: u32 = 1u32 << dds_status_id_DDS_SAMPLE_REJECTED_STATUS_ID;
/// Data on readers status bit (subscriber).
pub const STATUS_DATA_ON_READERS: u32 = 1u32 << dds_status_id_DDS_DATA_ON_READERS_STATUS_ID;
/// Data available status bit (reader).
pub const STATUS_DATA_AVAILABLE: u32 = 1u32 << dds_status_id_DDS_DATA_AVAILABLE_STATUS_ID;
/// Liveliness lost status bit (writer).
pub const STATUS_LIVELINESS_LOST: u32 = 1u32 << dds_status_id_DDS_LIVELINESS_LOST_STATUS_ID;
/// Liveliness changed status bit (reader).
pub const STATUS_LIVELINESS_CHANGED: u32 = 1u32 << dds_status_id_DDS_LIVELINESS_CHANGED_STATUS_ID;
/// Publication matched status bit (writer).
pub const STATUS_PUBLICATION_MATCHED: u32 =
    1u32 << dds_status_id_DDS_PUBLICATION_MATCHED_STATUS_ID;
/// Subscription matched status bit (reader).
pub const STATUS_SUBSCRIPTION_MATCHED: u32 =
    1u32 << dds_status_id_DDS_SUBSCRIPTION_MATCHED_STATUS_ID;

/// Convenience constant combining all status bits.
pub const STATUS_ALL: u32 = STATUS_INCONSISTENT_TOPIC
    | STATUS_OFFERED_DEADLINE_MISSED
    | STATUS_REQUESTED_DEADLINE_MISSED
    | STATUS_OFFERED_INCOMPATIBLE_QOS
    | STATUS_REQUESTED_INCOMPATIBLE_QOS
    | STATUS_SAMPLE_LOST
    | STATUS_SAMPLE_REJECTED
    | STATUS_DATA_ON_READERS
    | STATUS_DATA_AVAILABLE
    | STATUS_LIVELINESS_LOST
    | STATUS_LIVELINESS_CHANGED
    | STATUS_PUBLICATION_MATCHED
    | STATUS_SUBSCRIPTION_MATCHED;

// ---------------------------------------------------------------------------
// Sample-rejected reason enum
// ---------------------------------------------------------------------------

/// Reason a sample was rejected by a DataReader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleRejectedReason {
    /// The sample was not rejected.
    NotRejected,
    /// Rejected because the instances resource limit was reached.
    RejectedByInstancesLimit,
    /// Rejected because the samples resource limit was reached.
    RejectedBySamplesLimit,
    /// Rejected because the samples-per-instance resource limit was reached.
    RejectedBySamplesPerInstanceLimit,
    /// An unknown reason code from the C library.
    Unknown(u32),
}

impl From<dds_sample_rejected_status_kind> for SampleRejectedReason {
    fn from(v: dds_sample_rejected_status_kind) -> Self {
        match v {
            x if x == dds_sample_rejected_status_kind_DDS_NOT_REJECTED => {
                SampleRejectedReason::NotRejected
            }
            x if x == dds_sample_rejected_status_kind_DDS_REJECTED_BY_INSTANCES_LIMIT => {
                SampleRejectedReason::RejectedByInstancesLimit
            }
            x if x == dds_sample_rejected_status_kind_DDS_REJECTED_BY_SAMPLES_LIMIT => {
                SampleRejectedReason::RejectedBySamplesLimit
            }
            x if x == dds_sample_rejected_status_kind_DDS_REJECTED_BY_SAMPLES_PER_INSTANCE_LIMIT => {
                SampleRejectedReason::RejectedBySamplesPerInstanceLimit
            }
            other => SampleRejectedReason::Unknown(other),
        }
    }
}

// ---------------------------------------------------------------------------
// Typed status structs
// ---------------------------------------------------------------------------

/// Status for an inconsistent topic.
#[derive(Debug, Clone)]
pub struct InconsistentTopicStatus {
    /// Total cumulative count of inconsistent topics.
    pub total_count: u32,
    /// Change in count since the last time the status was read.
    pub total_count_change: i32,
}

/// Status for liveliness lost by a DataWriter.
#[derive(Debug, Clone)]
pub struct LivelinessLostStatus {
    /// Total cumulative count of liveliness lost events.
    pub total_count: u32,
    /// Change in count since the last time the status was read.
    pub total_count_change: i32,
}

/// Status for liveliness changes detected by a DataReader.
#[derive(Debug, Clone)]
pub struct LivelinessChangedStatus {
    /// Number of currently-alive data-writers.
    pub alive_count: u32,
    /// Number of currently-not-alive data-writers.
    pub not_alive_count: u32,
    /// Change in alive count since last read.
    pub alive_count_change: i32,
    /// Change in not-alive count since last read.
    pub not_alive_count_change: i32,
    /// Instance handle of the last data-writer whose liveliness changed.
    pub last_publication_handle: dds_instance_handle_t,
}

/// Status for an offered deadline missed by a DataWriter.
#[derive(Debug, Clone)]
pub struct OfferedDeadlineMissedStatus {
    /// Total cumulative count of missed deadlines.
    pub total_count: u32,
    /// Change in count since last read.
    pub total_count_change: i32,
    /// Instance handle of the last instance for which the deadline was missed.
    pub last_instance_handle: dds_instance_handle_t,
}

/// Status for an offered incompatible QoS by a DataWriter.
#[derive(Debug, Clone)]
pub struct OfferedIncompatibleQosStatus {
    /// Total cumulative count of incompatible QoS events.
    pub total_count: u32,
    /// Change in count since last read.
    pub total_count_change: i32,
    /// Policy ID of the last policy that was found to be incompatible.
    pub last_policy_id: u32,
}

/// Status for a requested deadline missed by a DataReader.
#[derive(Debug, Clone)]
pub struct RequestedDeadlineMissedStatus {
    /// Total cumulative count of missed deadlines.
    pub total_count: u32,
    /// Change in count since last read.
    pub total_count_change: i32,
    /// Instance handle of the last instance for which the deadline was missed.
    pub last_instance_handle: dds_instance_handle_t,
}

/// Status for a requested incompatible QoS by a DataReader.
#[derive(Debug, Clone)]
pub struct RequestedIncompatibleQosStatus {
    /// Total cumulative count of incompatible QoS events.
    pub total_count: u32,
    /// Change in count since last read.
    pub total_count_change: i32,
    /// Policy ID of the last policy that was found to be incompatible.
    pub last_policy_id: u32,
}

/// Status for samples lost by a DataReader.
#[derive(Debug, Clone)]
pub struct SampleLostStatus {
    /// Total cumulative count of lost samples.
    pub total_count: u32,
    /// Change in count since last read.
    pub total_count_change: i32,
}

/// Status for samples rejected by a DataReader.
#[derive(Debug, Clone)]
pub struct SampleRejectedStatus {
    /// Total cumulative count of rejected samples.
    pub total_count: u32,
    /// Change in count since last read.
    pub total_count_change: i32,
    /// Reason the last sample was rejected.
    pub last_reason: SampleRejectedReason,
    /// Instance handle of the last rejected sample.
    pub last_instance_handle: dds_instance_handle_t,
}

/// Status for publication matched (writer found a compatible reader).
#[derive(Debug, Clone)]
pub struct PublicationMatchedStatus {
    /// Total cumulative count of matched subscriptions.
    pub total_count: u32,
    /// Change in count since last read.
    pub total_count_change: i32,
    /// Current number of matched subscriptions.
    pub current_count: u32,
    /// Change in current count since last read.
    pub current_count_change: i32,
    /// Instance handle of the last matched subscription.
    pub last_subscription_handle: dds_instance_handle_t,
}

/// Status for subscription matched (reader found a compatible writer).
#[derive(Debug, Clone)]
pub struct SubscriptionMatchedStatus {
    /// Total cumulative count of matched publications.
    pub total_count: u32,
    /// Change in count since last read.
    pub total_count_change: i32,
    /// Current number of matched publications.
    pub current_count: u32,
    /// Change in current count since last read.
    pub current_count_change: i32,
    /// Instance handle of the last matched publication.
    pub last_publication_handle: dds_instance_handle_t,
}

// ---------------------------------------------------------------------------
// Conversion helpers – C struct → Rust struct
// ---------------------------------------------------------------------------

impl From<dds_inconsistent_topic_status> for InconsistentTopicStatus {
    fn from(s: dds_inconsistent_topic_status) -> Self {
        InconsistentTopicStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
        }
    }
}

impl From<dds_liveliness_lost_status> for LivelinessLostStatus {
    fn from(s: dds_liveliness_lost_status) -> Self {
        LivelinessLostStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
        }
    }
}

impl From<dds_liveliness_changed_status> for LivelinessChangedStatus {
    fn from(s: dds_liveliness_changed_status) -> Self {
        LivelinessChangedStatus {
            alive_count: s.alive_count,
            not_alive_count: s.not_alive_count,
            alive_count_change: s.alive_count_change,
            not_alive_count_change: s.not_alive_count_change,
            last_publication_handle: s.last_publication_handle,
        }
    }
}

impl From<dds_offered_deadline_missed_status> for OfferedDeadlineMissedStatus {
    fn from(s: dds_offered_deadline_missed_status) -> Self {
        OfferedDeadlineMissedStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
            last_instance_handle: s.last_instance_handle,
        }
    }
}

impl From<dds_offered_incompatible_qos_status> for OfferedIncompatibleQosStatus {
    fn from(s: dds_offered_incompatible_qos_status) -> Self {
        OfferedIncompatibleQosStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
            last_policy_id: s.last_policy_id,
        }
    }
}

impl From<dds_requested_deadline_missed_status> for RequestedDeadlineMissedStatus {
    fn from(s: dds_requested_deadline_missed_status) -> Self {
        RequestedDeadlineMissedStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
            last_instance_handle: s.last_instance_handle,
        }
    }
}

impl From<dds_requested_incompatible_qos_status> for RequestedIncompatibleQosStatus {
    fn from(s: dds_requested_incompatible_qos_status) -> Self {
        RequestedIncompatibleQosStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
            last_policy_id: s.last_policy_id,
        }
    }
}

impl From<dds_sample_lost_status> for SampleLostStatus {
    fn from(s: dds_sample_lost_status) -> Self {
        SampleLostStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
        }
    }
}

impl From<dds_sample_rejected_status> for SampleRejectedStatus {
    fn from(s: dds_sample_rejected_status) -> Self {
        SampleRejectedStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
            last_reason: SampleRejectedReason::from(s.last_reason),
            last_instance_handle: s.last_instance_handle,
        }
    }
}

impl From<dds_publication_matched_status> for PublicationMatchedStatus {
    fn from(s: dds_publication_matched_status) -> Self {
        PublicationMatchedStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
            current_count: s.current_count,
            current_count_change: s.current_count_change,
            last_subscription_handle: s.last_subscription_handle,
        }
    }
}

impl From<dds_subscription_matched_status> for SubscriptionMatchedStatus {
    fn from(s: dds_subscription_matched_status) -> Self {
        SubscriptionMatchedStatus {
            total_count: s.total_count,
            total_count_change: s.total_count_change,
            current_count: s.current_count,
            current_count_change: s.current_count_change,
            last_publication_handle: s.last_publication_handle,
        }
    }
}

// ---------------------------------------------------------------------------
// Extension trait – typed status getters on any DdsEntity
// ---------------------------------------------------------------------------

/// Extension trait that adds typed status-getter methods to any entity
/// implementing [`DdsEntity`].
///
/// Each method calls the corresponding `dds_get_*_status` C function,
/// maps errors through `crate::error::check`, and converts the raw C
/// struct into the appropriate typed Rust struct.
pub trait StatusExt: DdsEntity {
    /// Get the inconsistent-topic status (valid for Topic entities).
    ///
    /// Resets the trigger value.
    fn inconsistent_topic_status(&self) -> DdsResult<InconsistentTopicStatus> {
        unsafe {
            let mut raw: dds_inconsistent_topic_status_t = std::mem::zeroed();
            crate::error::check(dds_get_inconsistent_topic_status(
                self.entity(),
                &mut raw,
            ))?;
            Ok(InconsistentTopicStatus::from(raw))
        }
    }

    /// Get the liveliness-lost status (valid for DataWriter entities).
    ///
    /// Resets the trigger value.
    fn liveliness_lost_status(&self) -> DdsResult<LivelinessLostStatus> {
        unsafe {
            let mut raw: dds_liveliness_lost_status_t = std::mem::zeroed();
            crate::error::check(dds_get_liveliness_lost_status(self.entity(), &mut raw))?;
            Ok(LivelinessLostStatus::from(raw))
        }
    }

    /// Get the liveliness-changed status (valid for DataReader entities).
    ///
    /// Resets the trigger value.
    fn liveliness_changed_status(&self) -> DdsResult<LivelinessChangedStatus> {
        unsafe {
            let mut raw: dds_liveliness_changed_status_t = std::mem::zeroed();
            crate::error::check(dds_get_liveliness_changed_status(self.entity(), &mut raw))?;
            Ok(LivelinessChangedStatus::from(raw))
        }
    }

    /// Get the offered-deadline-missed status (valid for DataWriter entities).
    ///
    /// Resets the trigger value.
    fn offered_deadline_missed_status(&self) -> DdsResult<OfferedDeadlineMissedStatus> {
        unsafe {
            let mut raw: dds_offered_deadline_missed_status_t = std::mem::zeroed();
            crate::error::check(dds_get_offered_deadline_missed_status(
                self.entity(),
                &mut raw,
            ))?;
            Ok(OfferedDeadlineMissedStatus::from(raw))
        }
    }

    /// Get the offered-incompatible-QoS status (valid for DataWriter entities).
    ///
    /// Resets the trigger value.
    fn offered_incompatible_qos_status(&self) -> DdsResult<OfferedIncompatibleQosStatus> {
        unsafe {
            let mut raw: dds_offered_incompatible_qos_status_t = std::mem::zeroed();
            crate::error::check(dds_get_offered_incompatible_qos_status(
                self.entity(),
                &mut raw,
            ))?;
            Ok(OfferedIncompatibleQosStatus::from(raw))
        }
    }

    /// Get the requested-deadline-missed status (valid for DataReader entities).
    ///
    /// Resets the trigger value.
    fn requested_deadline_missed_status(&self) -> DdsResult<RequestedDeadlineMissedStatus> {
        unsafe {
            let mut raw: dds_requested_deadline_missed_status_t = std::mem::zeroed();
            crate::error::check(dds_get_requested_deadline_missed_status(
                self.entity(),
                &mut raw,
            ))?;
            Ok(RequestedDeadlineMissedStatus::from(raw))
        }
    }

    /// Get the requested-incompatible-QoS status (valid for DataReader entities).
    ///
    /// Resets the trigger value.
    fn requested_incompatible_qos_status(&self) -> DdsResult<RequestedIncompatibleQosStatus> {
        unsafe {
            let mut raw: dds_requested_incompatible_qos_status_t = std::mem::zeroed();
            crate::error::check(dds_get_requested_incompatible_qos_status(
                self.entity(),
                &mut raw,
            ))?;
            Ok(RequestedIncompatibleQosStatus::from(raw))
        }
    }

    /// Get the sample-lost status (valid for DataReader entities).
    ///
    /// Resets the trigger value.
    fn sample_lost_status(&self) -> DdsResult<SampleLostStatus> {
        unsafe {
            let mut raw: dds_sample_lost_status_t = std::mem::zeroed();
            crate::error::check(dds_get_sample_lost_status(self.entity(), &mut raw))?;
            Ok(SampleLostStatus::from(raw))
        }
    }

    /// Get the sample-rejected status (valid for DataReader entities).
    ///
    /// Resets the trigger value.
    fn sample_rejected_status(&self) -> DdsResult<SampleRejectedStatus> {
        unsafe {
            let mut raw: dds_sample_rejected_status_t = std::mem::zeroed();
            crate::error::check(dds_get_sample_rejected_status(self.entity(), &mut raw))?;
            Ok(SampleRejectedStatus::from(raw))
        }
    }

    /// Get the publication-matched status (valid for DataWriter entities).
    ///
    /// Resets the trigger value.
    fn publication_matched_status(&self) -> DdsResult<PublicationMatchedStatus> {
        unsafe {
            let mut raw: dds_publication_matched_status_t = std::mem::zeroed();
            crate::error::check(dds_get_publication_matched_status(
                self.entity(),
                &mut raw,
            ))?;
            Ok(PublicationMatchedStatus::from(raw))
        }
    }

    /// Get the subscription-matched status (valid for DataReader entities).
    ///
    /// Resets the trigger value.
    fn subscription_matched_status(&self) -> DdsResult<SubscriptionMatchedStatus> {
        unsafe {
            let mut raw: dds_subscription_matched_status_t = std::mem::zeroed();
            crate::error::check(dds_get_subscription_matched_status(
                self.entity(),
                &mut raw,
            ))?;
            Ok(SubscriptionMatchedStatus::from(raw))
        }
    }
}

// Blanket impl: every type that implements DdsEntity gets StatusExt for free.
impl<T: DdsEntity + ?Sized> StatusExt for T {}
