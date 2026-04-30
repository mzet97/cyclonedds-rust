#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(deref_nullptr)]
#![allow(clippy::all)]
#![allow(clippy::missing_safety_doc)]

use std::sync::atomic::{AtomicU32, Ordering};

pub use bindings::*;
mod bindings {
    use super::{ddsrt_atomic_uint32_t, ddsrt_byte_order_selector, ddsrt_hh, ddsrt_mtime_t};

    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub type ddsrt_hh_hash_fn =
    ::std::option::Option<unsafe extern "C" fn(a: *const ::std::ffi::c_void) -> u32>;
pub type ddsrt_hh_equals_fn = ::std::option::Option<
    unsafe extern "C" fn(a: *const ::std::ffi::c_void, b: *const ::std::ffi::c_void) -> bool,
>;
pub type ddsrt_hh_buckets_gc_fn = ::std::option::Option<
    unsafe extern "C" fn(bs: *mut ::std::ffi::c_void, arg: *mut ::std::ffi::c_void),
>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ddsrt_hh_bucket {
    pub hopinfo: u32,
    pub data: *mut ::std::ffi::c_void,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ddsrt_hh {
    pub size: u32,
    pub buckets: *mut ddsrt_hh_bucket,
    pub hash: ddsrt_hh_hash_fn,
    pub equals: ddsrt_hh_equals_fn,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ddsrt_hh_iter {
    pub hh: *mut ddsrt_hh,
    pub cursor: u32,
}

pub type ddsrt_byte_order_selector = ::std::ffi::c_uint;
pub const DDSRT_BOSEL_NATIVE: ddsrt_byte_order_selector = 0;
pub const DDSRT_BOSEL_BE: ddsrt_byte_order_selector = 1;
pub const DDSRT_BOSEL_LE: ddsrt_byte_order_selector = 2;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct ddsrt_atomic_uint32_t {
    pub v: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct ddsrt_mtime_t {
    pub v: dds_time_t,
}

#[inline]
pub const fn dds_err_nr(err: dds_return_t) -> dds_return_t {
    err
}

#[inline]
pub const fn dds_err_line(_err: dds_return_t) -> u32 {
    0
}

#[inline]
pub const fn dds_err_file_id(_err: dds_return_t) -> u32 {
    0
}

// NOTE: dds_loaned_sample and dds_rhc are opaque types in the Windows bindings
// (only _address field visible). These helpers are only available when the
// bindings expose the full struct layout (e.g., on macOS/Linux).
// They are not used by the cyclonedds crate directly.

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_loaned_sample_ref(loaned_sample: *mut dds_loaned_sample_t) {
    let refc = &(*loaned_sample).refc as *const ddsrt_atomic_uint32_t as *const AtomicU32;
    (*refc).fetch_add(1, Ordering::SeqCst);
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_loaned_sample_unref(loaned_sample: *mut dds_loaned_sample_t) {
    let refc = &(*loaned_sample).refc as *const ddsrt_atomic_uint32_t as *const AtomicU32;
    if (*refc).fetch_sub(1, Ordering::SeqCst) == 1 {
        if let Some(free_fn) = (*loaned_sample).ops.free {
            free_fn(loaned_sample);
        }
    }
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_associate(rhc: *mut dds_rhc, reader: *mut dds_reader) -> dds_return_t {
    ((*(*rhc).common.ops)
        .associate
        .expect("dds_rhc associate op"))(rhc, reader)
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_store(
    rhc: *mut dds_rhc,
    wrinfo: *const ddsi_writer_info,
    sample: *mut ddsi_serdata,
    tk: *mut ddsi_tkmap_instance,
) -> bool {
    ((*(*rhc).common.ops)
        .rhc_ops
        .store
        .expect("dds_rhc store op"))(
        std::ptr::addr_of_mut!((*rhc).common.rhc),
        wrinfo,
        sample,
        tk,
    )
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_unregister_wr(rhc: *mut dds_rhc, wrinfo: *const ddsi_writer_info) {
    ((*(*rhc).common.ops)
        .rhc_ops
        .unregister_wr
        .expect("dds_rhc unregister_wr op"))(std::ptr::addr_of_mut!((*rhc).common.rhc), wrinfo)
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_relinquish_ownership(rhc: *mut dds_rhc, wr_iid: u64) {
    ((*(*rhc).common.ops)
        .rhc_ops
        .relinquish_ownership
        .expect("dds_rhc relinquish_ownership op"))(
        std::ptr::addr_of_mut!((*rhc).common.rhc),
        wr_iid,
    )
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_free(rhc: *mut dds_rhc) {
    ((*(*rhc).common.ops).rhc_ops.free.expect("dds_rhc free op"))(std::ptr::addr_of_mut!(
        (*rhc).common.rhc
    ))
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_peek(
    rhc: *mut dds_rhc,
    max_samples: i32,
    mask: u32,
    handle: dds_instance_handle_t,
    cond: *mut dds_readcond,
    collect_sample: dds_read_with_collector_fn_t,
    collect_sample_arg: *mut ::std::ffi::c_void,
) -> i32 {
    ((*(*rhc).common.ops).peek.expect("dds_rhc peek op"))(
        rhc,
        max_samples,
        mask,
        handle,
        cond,
        collect_sample,
        collect_sample_arg,
    )
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_read(
    rhc: *mut dds_rhc,
    max_samples: i32,
    mask: u32,
    handle: dds_instance_handle_t,
    cond: *mut dds_readcond,
    collect_sample: dds_read_with_collector_fn_t,
    collect_sample_arg: *mut ::std::ffi::c_void,
) -> i32 {
    ((*(*rhc).common.ops).read.expect("dds_rhc read op"))(
        rhc,
        max_samples,
        mask,
        handle,
        cond,
        collect_sample,
        collect_sample_arg,
    )
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_take(
    rhc: *mut dds_rhc,
    max_samples: i32,
    mask: u32,
    handle: dds_instance_handle_t,
    cond: *mut dds_readcond,
    collect_sample: dds_read_with_collector_fn_t,
    collect_sample_arg: *mut ::std::ffi::c_void,
) -> i32 {
    ((*(*rhc).common.ops).take.expect("dds_rhc take op"))(
        rhc,
        max_samples,
        mask,
        handle,
        cond,
        collect_sample,
        collect_sample_arg,
    )
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_add_readcondition(rhc: *mut dds_rhc, cond: *mut dds_readcond) -> bool {
    ((*(*rhc).common.ops)
        .add_readcondition
        .expect("dds_rhc add_readcondition op"))(rhc, cond)
}

#[cfg(feature = "internal-ops")]
#[inline]
pub unsafe fn dds_rhc_remove_readcondition(rhc: *mut dds_rhc, cond: *mut dds_readcond) {
    ((*(*rhc).common.ops)
        .remove_readcondition
        .expect("dds_rhc remove_readcondition op"))(rhc, cond)
}

unsafe extern "C" {
    pub fn ddsi_sertype_ref(tp: *const ddsi_sertype) -> *mut ddsi_sertype;
    pub fn ddsi_sertype_unref(tp: *mut ddsi_sertype);
    pub fn ddsi_sertype_equal(a: *const ddsi_sertype, b: *const ddsi_sertype) -> bool;
    pub fn ddsi_sertype_hash(tp: *const ddsi_sertype) -> u32;
}

// ── ddsi_serdata vtable helpers ──
//
// The C headers define these as inline functions that dereference the serdata
// ops vtable.  Since bindgen produces an opaque `ddsi_serdata` struct, we
// re-implement them here by calling through the exported (non-inline)
// vtable-based helper, or – where that is not available – by reconstructing
// the offset arithmetic ourselves.
//
// The layout of `struct ddsi_serdata` (from ddsi_serdata.h):
//   offset 0:  ops    : *const ddsi_serdata_ops
//   offset 8:  hash   : u32
//   offset 12: refc   : ddsrt_atomic_uint32_t  (u32)
//   offset 16: kind   : c_int
//   offset 24: type   : *const ddsi_sertype
//   offset 32: timestamp
//   offset 40: statusinfo
//   ...
//   offset 56: loan   : *mut dds_loaned_sample
//
// The vtable (`ddsi_serdata_ops`, from ddsi_serdata.h):
//   offset 0:   eqkey
//   offset 8:   get_size
//   offset 16:  from_ser
//   offset 24:  from_ser_iov
//   offset 32:  from_keyhash
//   offset 40:  from_sample
//   offset 48:  to_ser
//   offset 56:  to_ser_ref
//   offset 64:  to_ser_unref
//   offset 72:  to_sample
//   offset 80:  to_untyped
//   offset 88:  untyped_to_sample
//   offset 96:  free
//   ...

/// Increment the reference count of a serdata, returning the same pointer.
///
/// # Safety
/// `d` must be a valid, non-null `ddsi_serdata` pointer.
#[inline]
pub unsafe fn ddsi_serdata_ref(d: *const ddsi_serdata) -> *mut ddsi_serdata {
    // refc is at offset 12 on 64-bit (after ops:8, hash:4)
    let refc_ptr = (d as *const u8).add(12) as *const AtomicU32;
    (*refc_ptr).fetch_add(1, Ordering::SeqCst);
    d as *mut ddsi_serdata
}

/// Decrement the reference count; free the serdata when it drops to zero.
///
/// # Safety
/// `d` must be a valid, non-null `ddsi_serdata` pointer that the caller
/// currently holds a reference to.
#[inline]
pub unsafe fn ddsi_serdata_unref(d: *mut ddsi_serdata) {
    // refc at offset 12
    let refc_ptr = (d as *const u8).add(12) as *const AtomicU32;
    if (*refc_ptr).fetch_sub(1, Ordering::SeqCst) == 1 {
        // ops at offset 0; free is at offset 96 in the vtable
        let ops = *(d as *const *const u8);
        let free_fn: unsafe extern "C" fn(*mut ddsi_serdata) =
            ::std::mem::transmute(*(ops as *const u8).add(96) as *const ::std::ffi::c_void);
        free_fn(d);
    }
}

/// Return the serialized size (in bytes) of a serdata.
///
/// # Safety
/// `d` must be a valid, non-null `ddsi_serdata` pointer.
#[inline]
pub unsafe fn ddsi_serdata_size(d: *const ddsi_serdata) -> u32 {
    let ops = *(d as *const *const u8);
    let get_size: unsafe extern "C" fn(*const ddsi_serdata) -> u32 =
        ::std::mem::transmute(*(ops as *const u8).add(8) as *const ::std::ffi::c_void);
    get_size(d)
}

/// Copy serialized bytes from a serdata into the provided buffer.
///
/// # Safety
/// `d` must be a valid, non-null `ddsi_serdata` pointer.
/// `buf` must point to a buffer of at least `sz` bytes.
#[inline]
pub unsafe fn ddsi_serdata_to_ser(d: *const ddsi_serdata, off: usize, sz: usize, buf: *mut ::std::ffi::c_void) {
    let ops = *(d as *const *const u8);
    let to_ser: unsafe extern "C" fn(*const ddsi_serdata, usize, usize, *mut ::std::ffi::c_void) =
        ::std::mem::transmute(*(ops as *const u8).add(48) as *const ::std::ffi::c_void);
    to_ser(d, off, sz, buf);
}

pub type ddsi_typeid_kind_t = ::std::ffi::c_int;
pub const DDSI_TYPEID_KIND_MINIMAL: ddsi_typeid_kind_t = 0;
pub const DDSI_TYPEID_KIND_COMPLETE: ddsi_typeid_kind_t = 1;
pub const DDSI_TYPEID_KIND_PLAIN_COLLECTION_MINIMAL: ddsi_typeid_kind_t = 2;
pub const DDSI_TYPEID_KIND_PLAIN_COLLECTION_COMPLETE: ddsi_typeid_kind_t = 3;
pub const DDSI_TYPEID_KIND_FULLY_DESCRIPTIVE: ddsi_typeid_kind_t = 4;
pub const DDSI_TYPEID_KIND_INVALID: ddsi_typeid_kind_t = 5;

unsafe extern "C" {
    pub fn ddsi_typeinfo_minimal_typeid(typeinfo: *const dds_typeinfo_t) -> *const dds_typeid_t;
    pub fn ddsi_typeinfo_complete_typeid(typeinfo: *const dds_typeinfo_t) -> *const dds_typeid_t;
    pub fn ddsi_typeinfo_present(typeinfo: *const dds_typeinfo_t) -> bool;
    pub fn ddsi_typeinfo_valid(typeinfo: *const dds_typeinfo_t) -> bool;
    pub fn ddsi_typeid_compare(a: *const dds_typeid_t, b: *const dds_typeid_t)
        -> ::std::ffi::c_int;
    pub fn ddsi_typeid_is_none(type_id: *const dds_typeid_t) -> bool;
    pub fn ddsi_typeid_is_hash(type_id: *const dds_typeid_t) -> bool;
    pub fn ddsi_typeid_is_minimal(type_id: *const dds_typeid_t) -> bool;
    pub fn ddsi_typeid_is_complete(type_id: *const dds_typeid_t) -> bool;
    pub fn ddsi_typeid_is_fully_descriptive(type_id: *const dds_typeid_t) -> bool;
    pub fn ddsi_typeid_kind(type_id: *const dds_typeid_t) -> ddsi_typeid_kind_t;
    pub fn ddsi_typeid_dup(src: *const dds_typeid_t) -> *mut dds_typeid_t;
    pub fn ddsi_typeid_fini(type_id: *mut dds_typeid_t);
}

pub type DDS_XTypes_EquivalenceHash = [u8; 14];

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ddsi_typeid_str {
    pub str_: [::std::ffi::c_char; 50usize],
}

pub type ddsi_type_include_deps_t = ::std::ffi::c_int;
pub const DDSI_TYPE_IGNORE_DEPS: ddsi_type_include_deps_t = 0;
pub const DDSI_TYPE_INCLUDE_DEPS: ddsi_type_include_deps_t = 1;

unsafe extern "C" {
    pub fn ddsi_typeinfo_equal(
        a: *const dds_typeinfo_t,
        b: *const dds_typeinfo_t,
        deps: ddsi_type_include_deps_t,
    ) -> bool;
    pub fn ddsi_typeinfo_typeid(
        type_info: *const dds_typeinfo_t,
        kind: ddsi_typeid_kind_t,
    ) -> *mut dds_typeid_t;
    pub fn ddsi_typeinfo_dup(src: *const dds_typeinfo_t) -> *mut dds_typeinfo_t;
    pub fn ddsi_typeinfo_free(typeinfo: *mut dds_typeinfo_t);
    pub fn ddsi_make_typeid_str(
        buf: *mut ddsi_typeid_str,
        type_id: *const dds_typeid_t,
    ) -> *mut ::std::ffi::c_char;
    pub fn ddsi_typeid_get_equivalence_hash(
        type_id: *const dds_typeid_t,
        hash: *mut DDS_XTypes_EquivalenceHash,
    );
}
