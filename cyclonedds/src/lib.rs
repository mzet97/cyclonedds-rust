#![allow(missing_docs)]

//! Safe, idiomatic Rust bindings for Eclipse CycloneDDS.
//!
//! CycloneDDS is a high-performance implementation of the OMG Data Distribution
//! Service (DDS) specification. This crate provides a safe Rust API on top of
//! the raw FFI bindings in `cyclonedds-rust-sys`.
//!
//! # Quick Start
//!
//! ```no_run
//! use cyclonedds::*;
//!
//! #[derive(DdsTypeDerive)]
//! struct HelloWorld {
//!     id: i32,
//!     message: DdsString,
//! }
//!
//! let participant = DomainParticipant::new(0).unwrap();
//! let topic = participant.create_topic::<HelloWorld>("HelloWorld").unwrap();
//! let publisher = participant.create_publisher().unwrap();
//! let writer = publisher.create_writer(&topic).unwrap();
//! let msg = HelloWorld {
//!     id: 1,
//!     message: DdsString::new("hello").unwrap(),
//! };
//! writer.write(&msg).unwrap();
//! ```

pub use cyclonedds_derive::DdsBitmask as DdsBitmaskDerive;
pub use cyclonedds_derive::DdsEnum as DdsEnumDerive;
pub use cyclonedds_derive::DdsType as DdsTypeDerive;
pub use cyclonedds_derive::DdsUnion as DdsUnionDerive;

#[cfg(feature = "async")]
#[allow(missing_docs)]
pub mod r#async;
#[allow(missing_docs)]
mod builtin;
#[allow(missing_docs)]
mod content_filtered_topic;
#[allow(missing_docs)]
mod dynamic_type;
#[allow(missing_docs)]
mod dynamic_value;
#[allow(missing_docs)]
mod entity;
#[allow(missing_docs)]
mod error;
#[allow(missing_docs)]
mod listener;
#[cfg(any(feature = "opentelemetry", feature = "tokio-console"))]
pub mod observability;
pub mod log;
mod participant;
mod participant_pool;
mod publisher;
mod qos;
mod request_reply;
mod qos_provider;
mod reader;
pub mod sample;
#[cfg(feature = "serde")]
mod serde_sample;
#[cfg(feature = "security")]
pub mod security;
mod sequence;
mod serialization;
mod statistics;
mod status;
mod string;
mod subscriber;
mod topic;
mod type_discovery;
mod waitset;
#[doc(hidden)]
pub mod write_arena;
mod writer;
mod xtypes;

pub use builtin::{
    BuiltinEndpointSample, BuiltinParticipantSample, BuiltinTopicSample,
    BUILTIN_TOPIC_DCPSPARTICIPANT, BUILTIN_TOPIC_DCPSPUBLICATION, BUILTIN_TOPIC_DCPSSUBSCRIPTION,
    BUILTIN_TOPIC_DCPSTOPIC, DDS_MIN_PSEUDO_HANDLE,
};
pub use content_filtered_topic::{ContentFilteredTopic, FilterParams, TopicFilterExt, TopicParameterizedFilterExt};
pub use dynamic_type::{
    DynamicEnumLiteralValue, DynamicMemberBuilder, DynamicPrimitiveKind, DynamicType,
    DynamicTypeAutoId, DynamicTypeBuilder, DynamicTypeExtensibility, DynamicTypeSpec,
};
pub use dynamic_value::{
    DynamicBitmaskFieldSchema, DynamicData, DynamicEnumLiteralSchema, DynamicFieldSchema,
    DynamicTypeSchema, DynamicUnionCaseSchema, DynamicValue,
};
pub use entity::DdsEntity;
pub use error::{err_file_id, err_line, err_nr, DdsError, DdsResult};
pub use listener::{Listener, ListenerBuilder};
pub use participant::DomainParticipant;
pub use publisher::Publisher;
pub use qos::{
    DataRepresentation, DestinationOrder, Durability, DurabilityServicePolicy, History,
    IgnoreLocalKind, Liveliness, Ownership, PresentationAccessScope, PresentationPolicy, Qos,
    QosBuilder, ReaderDataLifecyclePolicy, Reliability, TypeConsistency, TypeConsistencyPolicy,
};
pub use participant_pool::{DiscoveredParticipant, DiscoveredTopic, ParticipantPool};
pub use qos_provider::{QosKind, QosProvider};
pub use request_reply::{Replier, RequestReply, Requester};
pub use reader::DataReader;
pub use sample::{Loan, Sample};
#[cfg(feature = "serde")]
pub use serde_sample::SerdeSample;
#[cfg(feature = "security")]
pub use security::SecurityConfig;
pub use sequence::{DdsBoundedSequence, DdsSequence, DdsSequenceElement};
pub use serialization::{CdrDeserializer, CdrEncoding, CdrSample, CdrSerializer};
pub use statistics::{StatisticEntryRef, StatisticKind, StatisticValue, Statistics};
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
pub use string::DdsString;
pub use subscriber::Subscriber;
pub use topic::{
    adr, adr_bst, adr_key, rebase_ops, DdsEnumType, DdsType, DdsUnionType, DiscriminantType,
    KeyDescriptor, Topic, TopicKeyDescriptor, UntypedTopic, DDS_OP_MASK_CONST,
    DDS_OP_SUBTYPE_MASK_CONST, DDS_OP_TYPE_MASK_CONST, OP_ADR, OP_DLC, OP_FLAG_EXT, OP_FLAG_FP,
    OP_FLAG_KEY, OP_FLAG_MU, OP_FLAG_OPT, OP_FLAG_SGN, OP_FLAG_SZ_SHIFT, OP_JEQ4, OP_KOF, OP_MID,
    OP_RTS, SUBTYPE_1BY, SUBTYPE_2BY, SUBTYPE_4BY, SUBTYPE_8BY, SUBTYPE_BSQ, SUBTYPE_BST,
    SUBTYPE_ENU, SUBTYPE_SEQ, SUBTYPE_STR, SUBTYPE_STU, TYPE_1BY, TYPE_2BY, TYPE_4BY, TYPE_8BY,
    TYPE_ARR, TYPE_BSQ, TYPE_BST, TYPE_ENU, TYPE_EXT, TYPE_SEQ, TYPE_STR, TYPE_UNI,
};
pub use type_discovery::{
    cdr_to_dynamic_data, discover_all_publication_types, discover_all_subscription_types,
    discover_type_from_endpoint, discover_type_from_publication, discover_type_from_subscription,
    discover_type_from_type_info, dynamic_data_to_cdr, DiscoveredType,
};
pub use waitset::{set_active_qc, GuardCondition, QueryCondition, ReadCondition, WaitSet};
pub use writer::{DataWriter, WriteLoan};
pub use xtypes::{
    FindScope, MatchedEndpoint, MemberDescriptor, OwnedSertype, OwnedTypeId, SertypeHandle,
    TopicDescriptor, TypeDescriptor, TypeExtensibility, TypeIdKind, TypeIdRef, TypeIncludeDeps,
    TypeInfo, TypeKind, TypeObject,
};
