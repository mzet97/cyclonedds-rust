#![cfg_attr(feature = "no_std", no_std)]
#![allow(missing_docs)]

extern crate alloc;

/// Safe, idiomatic Rust bindings for Eclipse CycloneDDS.
#[cfg(not(feature = "no_std"))]
pub mod no_std_types;

#[cfg(feature = "no_std")]
pub mod no_std_types;

#[cfg(feature = "no_std")]
pub use no_std_types::*;

#[cfg(not(feature = "no_std"))]
pub use cyclonedds_derive::DdsBitmask as DdsBitmaskDerive;
#[cfg(not(feature = "no_std"))]
pub use cyclonedds_derive::DdsEnum as DdsEnumDerive;
#[cfg(not(feature = "no_std"))]
pub use cyclonedds_derive::DdsType as DdsTypeDerive;
#[cfg(not(feature = "no_std"))]
pub use cyclonedds_derive::DdsUnion as DdsUnionDerive;

#[cfg(all(feature = "async", not(feature = "no_std")))]
#[allow(missing_docs)]
pub mod r#async;
#[cfg(not(feature = "no_std"))]
#[allow(missing_docs)]
mod builtin;
#[cfg(not(feature = "no_std"))]
#[allow(missing_docs)]
mod content_filtered_topic;
#[cfg(not(feature = "no_std"))]
#[allow(missing_docs)]
mod dynamic_type;
#[cfg(not(feature = "no_std"))]
#[allow(missing_docs)]
mod dynamic_value;
#[cfg(not(feature = "no_std"))]
#[allow(missing_docs)]
mod entity;
#[allow(missing_docs)]
mod error;
#[cfg(not(feature = "no_std"))]
#[allow(missing_docs)]
mod listener;
#[cfg(all(any(feature = "opentelemetry", feature = "tokio-console"), not(feature = "no_std")))]
pub mod observability;
#[cfg(not(feature = "no_std"))]
pub mod log;
#[cfg(not(feature = "no_std"))]
mod participant;
#[cfg(not(feature = "no_std"))]
mod participant_pool;
#[cfg(not(feature = "no_std"))]
mod publisher;
#[cfg(not(feature = "no_std"))]
mod qos;
#[cfg(not(feature = "no_std"))]
mod request_reply;
#[cfg(not(feature = "no_std"))]
mod qos_provider;
#[cfg(not(feature = "no_std"))]
mod reader;
#[cfg(not(feature = "no_std"))]
pub mod sample;
#[cfg(all(feature = "serde", not(feature = "no_std")))]
mod serde_sample;
#[cfg(all(feature = "security", not(feature = "no_std")))]
pub mod security;
#[cfg(not(feature = "no_std"))]
mod sequence;
#[cfg(not(feature = "no_std"))]
mod serialization;
#[cfg(not(feature = "no_std"))]
mod statistics;
#[cfg(not(feature = "no_std"))]
mod status;
#[cfg(not(feature = "no_std"))]
mod string;
#[cfg(not(feature = "no_std"))]
mod subscriber;
#[cfg(not(feature = "no_std"))]
mod topic;
#[cfg(not(feature = "no_std"))]
mod type_discovery;
#[cfg(not(feature = "no_std"))]
mod waitset;
#[cfg(not(feature = "no_std"))]
#[doc(hidden)]
pub mod write_arena;
#[cfg(not(feature = "no_std"))]
mod writer;
#[cfg(not(feature = "no_std"))]
mod xtypes;

#[cfg(not(feature = "no_std"))]
pub use builtin::{
    BuiltinEndpointSample, BuiltinParticipantSample, BuiltinTopicSample,
    BUILTIN_TOPIC_DCPSPARTICIPANT, BUILTIN_TOPIC_DCPSPUBLICATION, BUILTIN_TOPIC_DCPSSUBSCRIPTION,
    BUILTIN_TOPIC_DCPSTOPIC, DDS_MIN_PSEUDO_HANDLE,
};
#[cfg(not(feature = "no_std"))]
pub use content_filtered_topic::{ContentFilteredTopic, FilterParams, TopicFilterExt, TopicParameterizedFilterExt};
#[cfg(not(feature = "no_std"))]
pub use dynamic_type::{
    DynamicEnumLiteralValue, DynamicMemberBuilder, DynamicPrimitiveKind, DynamicType,
    DynamicTypeAutoId, DynamicTypeBuilder, DynamicTypeExtensibility, DynamicTypeSpec,
};
#[cfg(not(feature = "no_std"))]
pub use dynamic_value::{
    DynamicBitmaskFieldSchema, DynamicData, DynamicEnumLiteralSchema, DynamicFieldSchema,
    DynamicTypeSchema, DynamicUnionCaseSchema, DynamicValue,
};
#[cfg(not(feature = "no_std"))]
pub use entity::DdsEntity;
#[cfg(not(feature = "no_std"))]
pub use error::{err_file_id, err_line, err_nr};
pub use error::{DdsError, DdsResult};
#[cfg(not(feature = "no_std"))]
pub use listener::{Listener, ListenerBuilder};
#[cfg(not(feature = "no_std"))]
pub use participant::DomainParticipant;
#[cfg(not(feature = "no_std"))]
pub use publisher::Publisher;
#[cfg(not(feature = "no_std"))]
pub use qos::{
    DataRepresentation, DestinationOrder, Durability, DurabilityServicePolicy, History,
    IgnoreLocalKind, Liveliness, Ownership, PresentationAccessScope, PresentationPolicy, Qos,
    QosBuilder, ReaderDataLifecyclePolicy, Reliability, TypeConsistency, TypeConsistencyPolicy,
};
#[cfg(not(feature = "no_std"))]
pub use participant_pool::{DiscoveredParticipant, DiscoveredTopic, ParticipantPool};
#[cfg(not(feature = "no_std"))]
pub use qos_provider::{QosKind, QosProvider};
#[cfg(not(feature = "no_std"))]
pub use request_reply::{Replier, RequestReply, Requester};
#[cfg(not(feature = "no_std"))]
pub use reader::DataReader;
#[cfg(not(feature = "no_std"))]
pub use sample::{Loan, Sample};
#[cfg(all(feature = "serde", not(feature = "no_std")))]
pub use serde_sample::SerdeSample;
#[cfg(all(feature = "security", not(feature = "no_std")))]
pub use security::SecurityConfig;
#[cfg(not(feature = "no_std"))]
pub use sequence::{DdsBoundedSequence, DdsSequence, DdsSequenceElement};
#[cfg(not(feature = "no_std"))]
pub use serialization::{CdrDeserializer, CdrEncoding, CdrSample, CdrSerializer};
#[cfg(not(feature = "no_std"))]
pub use statistics::{StatisticEntryRef, StatisticKind, StatisticValue, Statistics};
#[cfg(not(feature = "no_std"))]
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
#[cfg(not(feature = "no_std"))]
pub use string::DdsString;
#[cfg(not(feature = "no_std"))]
pub use subscriber::Subscriber;
#[cfg(not(feature = "no_std"))]
pub use topic::{
    adr, adr_bst, adr_key, rebase_ops, DdsEnumType, DdsType, DdsUnionType, DiscriminantType,
    KeyDescriptor, Topic, TopicKeyDescriptor, UntypedTopic, DDS_OP_MASK_CONST,
    DDS_OP_SUBTYPE_MASK_CONST, DDS_OP_TYPE_MASK_CONST, OP_ADR, OP_DLC, OP_FLAG_EXT, OP_FLAG_FP,
    OP_FLAG_KEY, OP_FLAG_MU, OP_FLAG_OPT, OP_FLAG_SGN, OP_FLAG_SZ_SHIFT, OP_JEQ4, OP_KOF, OP_MID,
    OP_RTS, SUBTYPE_1BY, SUBTYPE_2BY, SUBTYPE_4BY, SUBTYPE_8BY, SUBTYPE_BSQ, SUBTYPE_BST,
    SUBTYPE_ENU, SUBTYPE_SEQ, SUBTYPE_STU, SUBTYPE_STR, TYPE_1BY, TYPE_2BY, TYPE_4BY, TYPE_8BY,
    TYPE_ARR, TYPE_BSQ, TYPE_BST, TYPE_ENU, TYPE_EXT, TYPE_SEQ, TYPE_STR, TYPE_UNI,
};
#[cfg(not(feature = "no_std"))]
pub use type_discovery::{
    cdr_to_dynamic_data, discover_all_publication_types, discover_all_subscription_types,
    discover_type_from_endpoint, discover_type_from_publication, discover_type_from_subscription,
    discover_type_from_type_info, dynamic_data_to_cdr, DiscoveredType,
};
#[cfg(not(feature = "no_std"))]
pub use waitset::{set_active_qc, GuardCondition, QueryCondition, ReadCondition, WaitSet};
#[cfg(not(feature = "no_std"))]
pub use writer::{DataWriter, WriteLoan};
#[cfg(not(feature = "no_std"))]
pub use xtypes::{
    FindScope, MatchedEndpoint, MemberDescriptor, OwnedSertype, OwnedTypeId, SertypeHandle,
    TopicDescriptor, TypeDescriptor, TypeExtensibility, TypeIdKind, TypeIdRef, TypeIncludeDeps,
    TypeInfo, TypeKind, TypeObject,
};
