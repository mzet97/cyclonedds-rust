//! Manual FFI bindings for CycloneDDS
//!
//! This file contains minimal manual bindings to avoid build-time
//! dependency on clang/libclang.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::os::raw::c_void;

// Basic types
pub type dds_entity_t = u32;
pub type dds_return_t = i32;
pub type dds_instance_handle_t = u64;
pub type dds_duration_t = i64;

// Sample info structure
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct dds_sample_info_t {
    pub sample_state: u32,
    pub view_state: u32,
    pub instance_state: u32,
    pub source_timestamp: dds_duration_t,
    pub instance_handle: dds_instance_handle_t,
    pub publication_handle: dds_instance_handle_t,
}

// Sample states
pub const DDS_READ_SAMPLE_STATE: u32 = 1;
pub const DDS_NOT_READ_SAMPLE_STATE: u32 = 2;
pub const DDS_ANY_SAMPLE_STATE: u32 = 3;
pub const DDS_READ_SAMPLE_STATE_ANY: u32 = 3;

// View states
pub const DDS_NEW_VIEW_STATE: u32 = 1;
pub const DDS_NOT_NEW_VIEW_STATE: u32 = 2;
pub const DDS_VIEW_STATE_ANY: u32 = 3;

// Instance states
pub const DDS_ALIVE_INSTANCE_STATE: u32 = 1;
pub const DDS_NOT_ALIVE_DISPOSED_INSTANCE_STATE: u32 = 2;
pub const DDS_NOT_ALIVE_NO_WRITERS_INSTANCE_STATE: u32 = 4;
pub const DDS_ANY_INSTANCE_STATE: u32 = 7;

// Return codes
pub const DDS_RETCODE_OK: dds_return_t = 0;
pub const DDS_RETCODE_ERROR: dds_return_t = -1;
pub const DDS_RETCODE_OUT_OF_RESOURCES: dds_return_t = -1;
pub const DDS_RETCODE_OUT_OF_MEMORY: dds_return_t = -2;
pub const DDS_RETCODE_BAD_PARAMETER: dds_return_t = -3;
pub const DDS_RETCODE_PRECONDITION_NOT_MET: dds_return_t = -4;
pub const DDS_RETCODE_OUT_OF_RESOURCES_ERROR: dds_return_t = -5;
pub const DDS_RETCODE_NOT_ENABLED: dds_return_t = -6;
pub const DDS_RETCODE_IMMUTABLE_POLICY: dds_return_t = -7;
pub const DDS_RETCODE_INCONSISTENT_POLICY: dds_return_t = -8;
pub const DDS_RETCODE_ALREADY_DELETED: dds_return_t = -9;
pub const DDS_RETCODE_TIMEOUT: dds_return_t = -10;
pub const DDS_RETCODE_NO_DATA: dds_return_t = -11;
pub const DDS_RETCODE_UNSUPPORTED: dds_return_t = -12;

// WaitSet
#[repr(C)]
#[derive(Debug, Clone)]
pub struct dds_waitset {
    pub trigger: u32,
    pub events: u32,
    pub nthreads: usize,
    pub threads: *mut c_void,
}

// External function declarations
extern "C" {
    // Participant functions
    pub fn dds_create_participant(
        domainid: u32,
        qos: *const c_void,
        listener: *const c_void,
    ) -> dds_entity_t;

    pub fn dds_delete(entity: dds_entity_t) -> dds_return_t;

    // Topic functions
    pub fn dds_create_topic(
        participant: dds_entity_t,
        type_name: *const i8,
        topic_name: *const i8,
        qos: *const c_void,
        listener: *const c_void,
    ) -> dds_entity_t;

    // Publisher functions
    pub fn dds_create_publisher(
        participant: dds_entity_t,
        qos: *const c_void,
        listener: *const c_void,
    ) -> dds_entity_t;

    // Subscriber functions
    pub fn dds_create_subscriber(
        participant: dds_entity_t,
        qos: *const c_void,
        listener: *const c_void,
    ) -> dds_entity_t;

    // DataWriter functions
    pub fn dds_create_writer(
        publisher: dds_entity_t,
        topic: dds_entity_t,
        qos: *const c_void,
        listener: *const c_void,
    ) -> dds_entity_t;

    pub fn dds_write(writer: dds_entity_t, data: *const c_void) -> dds_return_t;

    // DataReader functions
    pub fn dds_create_reader(
        subscriber: dds_entity_t,
        topic: dds_entity_t,
        qos: *const c_void,
        listener: *const c_void,
    ) -> dds_entity_t;

    pub fn dds_read(
        reader: dds_entity_t,
        samples: *mut *mut c_void,
        sample_info: *mut *mut dds_sample_info_t,
        min_samples: usize,
        max_samples: usize,
        sample_states: u32,
        view_states: u32,
        instance_states: u32,
    ) -> dds_return_t;

    pub fn dds_take(
        reader: dds_entity_t,
        samples: *mut *mut c_void,
        sample_info: *mut *mut dds_sample_info_t,
        min_samples: usize,
        max_samples: usize,
        sample_states: u32,
        view_states: u32,
        instance_states: u32,
    ) -> dds_return_t;

    // WaitSet functions
    pub fn dds_waitset_create() -> dds_waitset;

    pub fn dds_waitset_attach(
        ws: *mut dds_waitset,
        entity: dds_entity_t,
        entity_idx: u32,
    ) -> dds_return_t;

    pub fn dds_waitset_wait(
        ws: *mut dds_waitset,
        ret_entities: *mut dds_entity_t,
        ns: dds_duration_t,
    ) -> dds_return_t;

    // Initialization
    pub fn dds_init(domain_id: u32) -> dds_return_t;
    pub fn dds_fini() -> dds_return_t;
}
