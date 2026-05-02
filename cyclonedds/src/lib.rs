#![cfg_attr(all(feature = "no_std", not(feature = "std")), no_std)]
#![allow(missing_docs)]

extern crate alloc;

/// Safe, idiomatic Rust bindings for Eclipse CycloneDDS.
pub mod no_std_types;

#[cfg(not(feature = "std"))]
pub use no_std_types::*;

#[cfg(feature = "std")]
pub use cyclonedds_derive::DdsBitmask as DdsBitmaskDerive;
#[cfg(feature = "std")]
pub use cyclonedds_derive::DdsEnum as DdsEnumDerive;
#[cfg(feature = "std")]
pub use cyclonedds_derive::DdsType as DdsTypeDerive;
#[cfg(feature = "std")]
pub use cyclonedds_derive::DdsUnion as DdsUnionDerive;

#[cfg(all(feature = "async", feature = "std"))]
#[allow(missing_docs)]
pub mod r#async;
#[cfg(feature = "std")]
#[allow(missing_docs)]
mod builtin;
#[cfg(feature = "std")]
#[allow(missing_docs)]
mod content_filtered_topic;
#[cfg(feature = "std")]
#[allow(missing_docs)]
mod dynamic_type;
#[cfg(feature = "std")]
#[allow(missing_docs)]
mod dynamic_value;
#[cfg(feature = "std")]
#[allow(missing_docs)]
mod entity;
#[allow(missing_docs)]
mod error;
#[cfg(feature = "std")]
#[allow(missing_docs)]
mod listener;
#[cfg(all(any(feature = "opentelemetry", feature = "tokio-console"), feature = "std"))]
pub mod observability;
#[cfg(feature = "std")]
pub mod log;
#[cfg(feature = "std")]
mod participant;
#[cfg(feature = "std")]
mod participant_pool;
#[cfg(feature = "std")]
mod publisher;
#[cfg(feature = "std")]
mod qos;
#[cfg(feature = "std")]
mod request_reply;
#[cfg(feature = "std")]
mod qos_provider;
#[cfg(feature = "std")]
mod reader;
#[cfg(feature = "std")]
pub mod sample;
#[cfg(all(feature = "serde", feature = "std"))]
mod serde_sample;
#[cfg(all(feature = "security", feature = "std"))]
pub mod security;
#[cfg(feature = "std")]
mod sequence;
#[cfg(feature = "std")]
mod serialization;
#[cfg(feature = "std")]
mod statistics;
#[cfg(feature = "std")]
mod status;
#[cfg(feature = "std")]
mod string;
#[cfg(feature = "std")]
mod subscriber;
#[cfg(feature = "std")]
mod topic;
#[cfg(feature = "std")]
mod type_discovery;
#[cfg(feature = "std")]
mod waitset;
#[cfg(feature = "std")]
#[doc(hidden)]
pub mod write_arena;
#[cfg(feature = "std")]
mod writer;
#[cfg(feature = "std")]
mod xtypes;

#[cfg(feature = "std")]
pub use builtin::{
    BuiltinEndpointSample, BuiltinParticipantSample, BuiltinTopicSample,
    BUILTIN_TOPIC_DCPSPARTICIPANT, BUILTIN_TOPIC_DCPSPUBLICATION, BUILTIN_TOPIC_DCPSSUBSCRIPTION,
    BUILTIN_TOPIC_DCPSTOPIC, DDS_MIN_PSEUDO_HANDLE,
};
#[cfg(feature = "std")]
pub use content_filtered_topic::{ContentFilteredTopic, FilterParams, TopicFilterExt, TopicParameterizedFilterExt};
#[cfg(feature = "std")]
pub use dynamic_type::{
    DynamicEnumLiteralValue, DynamicMemberBuilder, DynamicPrimitiveKind, DynamicType,
    DynamicTypeAutoId, DynamicTypeBuilder, DynamicTypeExtensibility, DynamicTypeSpec,
};
#[cfg(feature = "std")]
pub use dynamic_value::{
    DynamicBitmaskFieldSchema, DynamicData, DynamicEnumLiteralSchema, DynamicFieldSchema,
    DynamicTypeSchema, DynamicUnionCaseSchema, DynamicValue,
};
#[cfg(feature = "std")]
pub use entity::DdsEntity;
#[cfg(feature = "std")]
#[cfg(feature = "std")]
pub use error::{err_file_id, err_line, err_nr};
pub use error::{DdsError, DdsResult};
#[cfg(feature = "std")]
pub use listener::{Listener, ListenerBuilder};
#[cfg(feature = "std")]
pub use participant::DomainParticipant;
#[cfg(feature = "std")]
pub use publisher::Publisher;
#[cfg(feature = "std")]
pub use qos::{
    DataRepresentation, DestinationOrder, Durability, DurabilityServicePolicy, History,
    IgnoreLocalKind, Liveliness, Ownership, PresentationAccessScope, PresentationPolicy, Qos,
    QosBuilder, ReaderDataLifecyclePolicy, Reliability, TypeConsistency, TypeConsistencyPolicy,
};
#[cfg(feature = "std")]
pub use participant_pool::{DiscoveredParticipant, DiscoveredTopic, ParticipantPool};
#[cfg(feature = "std")]
pub use qos_provider::{QosKind, QosProvider};
#[cfg(feature = "std")]
pub use request_reply::{Replier, RequestReply, Requester};
#[cfg(feature = "std")]
pub use reader::DataReader;
#[cfg(feature = "std")]
pub use sample::{Loan, Sample};
#[cfg(all(feature = "serde", feature = "std"))]
pub use serde_sample::SerdeSample;
#[cfg(all(feature = "security", feature = "std"))]
pub use security::SecurityConfig;
#[cfg(feature = "std")]
pub use sequence::{DdsBoundedSequence, DdsSequence, DdsSequenceElement};
#[cfg(feature = "std")]
pub use serialization::{CdrDeserializer, CdrEncoding, CdrSample, CdrSerializer};
#[cfg(feature = "std")]
pub use statistics::{StatisticEntryRef, StatisticKind, StatisticValue, Statistics};
#[cfg(feature = "std")]
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
#[cfg(feature = "std")]
pub use string::DdsString;
#[cfg(feature = "std")]
pub use subscriber::Subscriber;
#[cfg(feature = "std")]
pub use topic::{
    adr, adr_bst, adr_key, rebase_ops, DdsEnumType, DdsType, DdsUnionType, DiscriminantType,
    KeyDescriptor, Topic, TopicKeyDescriptor, UntypedTopic, DDS_OP_MASK_CONST,
    DDS_OP_SUBTYPE_MASK_CONST, DDS_OP_TYPE_MASK_CONST, OP_ADR, OP_DLC, OP_FLAG_EXT, OP_FLAG_FP,
    OP_FLAG_KEY, OP_FLAG_MU, OP_FLAG_OPT, OP_FLAG_SGN, OP_FLAG_SZ_SHIFT, OP_JEQ4, OP_KOF, OP_MID,
    OP_RTS, SUBTYPE_1BY, SUBTYPE_2BY, SUBTYPE_4BY, SUBTYPE_8BY, SUBTYPE_BSQ, SUBTYPE_BST,
    SUBTYPE_ENU, SUBTYPE_SEQ, SUBTYPE_STU, SUBTYPE_STR, TYPE_1BY, TYPE_2BY, TYPE_4BY, TYPE_8BY,
    TYPE_ARR, TYPE_BSQ, TYPE_BST, TYPE_ENU, TYPE_EXT, TYPE_SEQ, TYPE_STR, TYPE_UNI,
};
#[cfg(feature = "std")]
pub use type_discovery::{
    cdr_to_dynamic_data, discover_all_publication_types, discover_all_subscription_types,
    discover_type_from_endpoint, discover_type_from_publication, discover_type_from_subscription,
    discover_type_from_type_info, dynamic_data_to_cdr, DiscoveredType,
};
#[cfg(feature = "std")]
pub use waitset::{set_active_qc, GuardCondition, QueryCondition, ReadCondition, WaitSet};
#[cfg(feature = "std")]
pub use writer::{DataWriter, WriteLoan};
#[cfg(feature = "std")]
pub use xtypes::{
    FindScope, MatchedEndpoint, MemberDescriptor, OwnedSertype, OwnedTypeId, SertypeHandle,
    TopicDescriptor, TypeDescriptor, TypeExtensibility, TypeIdKind, TypeIdRef, TypeIncludeDeps,
    TypeInfo, TypeKind, TypeObject,
};
