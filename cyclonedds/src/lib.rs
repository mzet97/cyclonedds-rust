#![cfg_attr(all(feature = "no_std", not(feature = "std")), no_std)]
#![allow(missing_docs)]

extern crate alloc;

/// Safe, idiomatic Rust bindings for Eclipse CycloneDDS.
pub mod no_std_types;

#[cfg(not(feature = "std"))]
pub use no_std_types::*;

// Derive macros are pure proc-macros (portable type contracts): they stay
// on `std` and must keep working without the native backend.
#[cfg(feature = "std")]
pub use cyclonedds_derive::DdsBitmask as DdsBitmaskDerive;
#[cfg(feature = "std")]
pub use cyclonedds_derive::DdsEnum as DdsEnumDerive;
#[cfg(feature = "std")]
pub use cyclonedds_derive::DdsType as DdsTypeDerive;
#[cfg(feature = "std")]
pub use cyclonedds_derive::DdsUnion as DdsUnionDerive;

// Everything below needs the native `libddsc` backend
// (`cyclonedds-rust-sys`): it is gated on `native`, never on `std` alone,
// so a web/WASI consumer with `default-features = false, features = ["std"]`
// cannot accidentally link native DDS.
#[cfg(all(feature = "async", feature = "native"))]
#[allow(missing_docs)]
pub mod r#async;
#[cfg(feature = "native")]
#[allow(missing_docs)]
mod builtin;
#[cfg(feature = "native")]
#[allow(missing_docs)]
mod content_filtered_topic;
#[cfg(feature = "native")]
#[allow(missing_docs)]
mod dynamic_type;
#[cfg(feature = "native")]
#[allow(missing_docs)]
mod dynamic_value;
#[cfg(feature = "native")]
#[allow(missing_docs)]
mod entity;
#[allow(missing_docs)]
mod error;
#[cfg(feature = "native")]
#[allow(missing_docs)]
mod listener;
#[cfg(feature = "native")]
pub mod log;
#[cfg(all(
    any(feature = "opentelemetry", feature = "tokio-console"),
    feature = "native"
))]
pub mod observability;
#[cfg(feature = "native")]
mod participant;
#[cfg(feature = "native")]
mod participant_pool;
#[cfg(feature = "native")]
mod publisher;
#[cfg(feature = "native")]
mod qos;
#[cfg(feature = "native")]
mod qos_provider;
#[cfg(feature = "native")]
mod reader;
#[cfg(feature = "native")]
mod request_reply;
#[cfg(feature = "native")]
pub mod sample;
#[cfg(all(feature = "security", feature = "native"))]
pub mod security;
#[cfg(feature = "native")]
mod sequence;
#[cfg(all(feature = "serde", feature = "native"))]
mod serde_sample;
#[cfg(feature = "native")]
mod serialization;
#[cfg(feature = "native")]
mod statistics;
#[cfg(feature = "native")]
mod status;
#[cfg(feature = "native")]
mod string;
#[cfg(feature = "native")]
mod subscriber;
#[cfg(feature = "native")]
mod topic;
#[cfg(feature = "native")]
mod type_discovery;
#[cfg(feature = "native")]
mod waitset;
#[cfg(feature = "native")]
#[doc(hidden)]
pub mod write_arena;
#[cfg(feature = "native")]
mod writer;
#[cfg(feature = "native")]
mod xtypes;

#[cfg(feature = "native")]
pub use builtin::{
    BuiltinEndpointSample, BuiltinParticipantSample, BuiltinTopicSample,
    BUILTIN_TOPIC_DCPSPARTICIPANT, BUILTIN_TOPIC_DCPSPUBLICATION, BUILTIN_TOPIC_DCPSSUBSCRIPTION,
    BUILTIN_TOPIC_DCPSTOPIC, DDS_MIN_PSEUDO_HANDLE,
};
#[cfg(feature = "native")]
pub use content_filtered_topic::{
    ContentFilteredTopic, FilterParams, TopicFilterExt, TopicParameterizedFilterExt,
};
#[cfg(feature = "native")]
pub use dynamic_type::{
    DynamicEnumLiteralValue, DynamicMemberBuilder, DynamicPrimitiveKind, DynamicType,
    DynamicTypeAutoId, DynamicTypeBuilder, DynamicTypeExtensibility, DynamicTypeSpec,
};
#[cfg(feature = "native")]
pub use dynamic_value::{
    DynamicBitmaskFieldSchema, DynamicData, DynamicEnumLiteralSchema, DynamicFieldSchema,
    DynamicTypeSchema, DynamicUnionCaseSchema, DynamicValue,
};
#[cfg(feature = "native")]
pub use entity::DdsEntity;
#[cfg(feature = "native")]
pub use error::{err_file_id, err_line, err_nr};
pub use error::{DdsError, DdsResult};
#[cfg(feature = "native")]
pub use listener::{Listener, ListenerBuilder};
#[cfg(feature = "native")]
pub use participant::DomainParticipant;
#[cfg(feature = "native")]
pub use participant_pool::{DiscoveredParticipant, DiscoveredTopic, ParticipantPool};
#[cfg(feature = "native")]
pub use publisher::Publisher;
#[cfg(feature = "native")]
pub use qos::{
    DataRepresentation, DestinationOrder, Durability, DurabilityServicePolicy, History,
    IgnoreLocalKind, Liveliness, Ownership, PresentationAccessScope, PresentationPolicy, Qos,
    QosBuilder, ReaderDataLifecyclePolicy, Reliability, TypeConsistency, TypeConsistencyPolicy,
};
#[cfg(feature = "native")]
pub use qos_provider::{QosKind, QosProvider};
#[cfg(feature = "native")]
pub use reader::DataReader;
#[cfg(feature = "native")]
pub use request_reply::{Replier, RequestReply, Requester};
#[cfg(feature = "native")]
pub use sample::{Loan, Sample};
#[cfg(all(feature = "security", feature = "native"))]
pub use security::SecurityConfig;
#[cfg(feature = "native")]
pub use sequence::{DdsBoundedSequence, DdsSequence, DdsSequenceElement};
#[cfg(all(feature = "serde", feature = "native"))]
pub use serde_sample::{SerdeSample, SerdeTypeName};
#[cfg(feature = "native")]
pub use serialization::{CdrDeserializer, CdrEncoding, CdrSample, CdrSerializer};
#[cfg(feature = "native")]
pub use statistics::{StatisticEntryRef, StatisticKind, StatisticValue, Statistics};
#[cfg(feature = "native")]
pub use status::{
    EntityStatus, InconsistentTopicStatus, LivelinessChangedStatus, LivelinessLostStatus,
    OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus, PublicationMatchedStatus,
    RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus, SampleLostStatus,
    SampleRejectedReason, SampleRejectedStatus, StatusExt, SubscriptionMatchedStatus, STATUS_ALL,
    STATUS_DATA_AVAILABLE, STATUS_DATA_ON_READERS, STATUS_INCONSISTENT_TOPIC,
    STATUS_LIVELINESS_CHANGED, STATUS_LIVELINESS_LOST, STATUS_OFFERED_DEADLINE_MISSED,
    STATUS_OFFERED_INCOMPATIBLE_QOS, STATUS_PUBLICATION_MATCHED, STATUS_REQUESTED_DEADLINE_MISSED,
    STATUS_REQUESTED_INCOMPATIBLE_QOS, STATUS_SAMPLE_LOST, STATUS_SAMPLE_REJECTED,
    STATUS_SUBSCRIPTION_MATCHED,
};
#[cfg(feature = "native")]
pub use string::DdsString;
#[cfg(feature = "native")]
pub use subscriber::Subscriber;
#[cfg(feature = "native")]
pub use topic::{
    adr, adr_bst, adr_key, rebase_ops, DdsEnumType, DdsNativeValue, DdsType, DdsUnionType,
    DiscriminantType, KeyDescriptor, Topic, TopicKeyDescriptor, UntypedTopic, DDS_OP_MASK_CONST,
    DDS_OP_SUBTYPE_MASK_CONST, DDS_OP_TYPE_MASK_CONST, OP_ADR, OP_DLC, OP_FLAG_DEF, OP_FLAG_EXT,
    OP_FLAG_FP, OP_FLAG_KEY, OP_FLAG_MU, OP_FLAG_OPT, OP_FLAG_SGN, OP_FLAG_SZ_SHIFT, OP_JEQ4,
    OP_KOF, OP_MID, OP_RTS, SUBTYPE_1BY, SUBTYPE_2BY, SUBTYPE_4BY, SUBTYPE_8BY, SUBTYPE_BSQ,
    SUBTYPE_BST, SUBTYPE_ENU, SUBTYPE_SEQ, SUBTYPE_STR, SUBTYPE_STU, TYPE_1BY, TYPE_2BY, TYPE_4BY,
    TYPE_8BY, TYPE_ARR, TYPE_BSQ, TYPE_BST, TYPE_ENU, TYPE_EXT, TYPE_SEQ, TYPE_STR, TYPE_UNI,
};
#[cfg(feature = "native")]
pub use type_discovery::{
    cdr_to_dynamic_data, discover_all_publication_types, discover_all_subscription_types,
    discover_type_from_endpoint, discover_type_from_publication, discover_type_from_subscription,
    discover_type_from_type_info, dynamic_data_to_cdr, DiscoveredType,
};
#[cfg(feature = "native")]
pub use waitset::{GuardCondition, QcGuard, QueryCondition, ReadCondition, WaitSet};
#[cfg(feature = "native")]
pub use writer::{DataWriter, WriteLoan};
#[cfg(feature = "native")]
pub use xtypes::{
    FindScope, MatchedEndpoint, MemberDescriptor, OwnedSertype, OwnedTypeId, SertypeHandle,
    TopicDescriptor, TypeDescriptor, TypeExtensibility, TypeIdKind, TypeIdRef, TypeIncludeDeps,
    TypeInfo, TypeKind, TypeObject,
};
