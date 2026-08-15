//! `DdsError::from(retcode)` against the C's own table.
//!
//! Reference: `vendor/cyclonedds/src/ddsrt/include/dds/ddsrt/retcode.h:32-45`.
//!
//! ```text
//!   0  OK                      -7  IMMUTABLE_POLICY
//!  -1  ERROR                   -8  INCONSISTENT_POLICY
//!  -2  UNSUPPORTED             -9  ALREADY_DELETED
//!  -3  BAD_PARAMETER          -10  TIMEOUT
//!  -4  PRECONDITION_NOT_MET   -11  NO_DATA
//!  -5  OUT_OF_RESOURCES       -12  ILLEGAL_OPERATION
//!  -6  NOT_ENABLED            -13  NOT_ALLOWED_BY_SECURITY
//! ```
//!
//! There is no "out of memory" retcode in CycloneDDS at all: `ddsrt_malloc`
//! aborts on allocation failure rather than returning null. `DdsError::OutOfMemory`
//! is a Rust-side condition (a `checked_mul` overflow in the sequence
//! constructors) and must not be produced by the retcode conversion.

use cyclonedds::{
    DdsError, DomainParticipant, DynamicPrimitiveKind, DynamicTypeBuilder, DynamicTypeSpec,
};

#[test]
fn retcode_minus_two_is_unsupported_not_out_of_memory() {
    let err = DdsError::from(-2);
    assert!(
        matches!(err, DdsError::Unsupported(_)),
        "-2 is DDS_RETCODE_UNSUPPORTED, got {err:?}"
    );
}

#[test]
fn retcode_minus_twelve_is_illegal_operation_not_unsupported() {
    let err = DdsError::from(-12);
    assert!(
        !matches!(err, DdsError::Unsupported(_)),
        "-12 is DDS_RETCODE_ILLEGAL_OPERATION, not UNSUPPORTED; got {err:?}"
    );
    assert!(
        err.to_string().contains("illegal operation"),
        "message should name the retcode, got {err}"
    );
}

#[test]
fn retcode_minus_thirteen_is_mapped() {
    let err = DdsError::from(-13);
    assert!(
        err.to_string().contains("security"),
        "-13 is DDS_RETCODE_NOT_ALLOWED_BY_SECURITY, got {err}"
    );
}

#[test]
fn out_of_memory_has_no_retcode() {
    // -2 belongs to UNSUPPORTED, so round-tripping OutOfMemory through it would
    // turn a Rust allocation failure into "feature unsupported" and back.
    assert_eq!(DdsError::OutOfMemory.raw_code(), None);
}

/// The consequence, end to end.
///
/// CycloneDDS 11.0.1 does not implement dynamic maps —
/// `dds_dynamic_type.c:237` returns `DDS_RETCODE_UNSUPPORTED` for
/// `DDS_DYNAMIC_MAP`. That is a permanent condition, but it used to arrive as
/// `DdsError::OutOfMemory`, for which `is_transient()` answers `true`: a caller
/// retrying transient failures would retry this one forever.
#[test]
fn unsupported_dynamic_map_is_not_reported_as_transient() {
    let participant = DomainParticipant::new(0).unwrap();
    let i32_spec = || DynamicTypeSpec::primitive(DynamicPrimitiveKind::Int32);

    let err = DynamicTypeBuilder::map("M", i32_spec(), i32_spec(), None)
        .build(&participant)
        .expect_err("CycloneDDS 11 does not support dynamic maps");

    assert!(
        matches!(err, DdsError::Unsupported(_)),
        "expected Unsupported, got {err:?}"
    );
    assert!(
        !err.is_transient(),
        "an unsupported feature is never worth retrying"
    );
}
