//! A null or misaligned sample pointer must be an error, not a dereference.
//!
//! The default `DdsType::clone_out` and the derive-generated implementations
//! reject invalid pointers with `DdsError::BadParameter`. The hand-written
//! implementations for the built-in topic types (and `SerdeSample`) skipped
//! that guard and dereferenced the pointer directly, which is the defect
//! behind CodeQL alerts #129-#133 (`rust/access-invalid-pointer`).

use cyclonedds::*;

// SAFETY: each call below passes a deliberately invalid pointer to an
// `unsafe fn` whose contract now requires it to validate the pointer and
// return `Err` without dereferencing it. No valid sample is read.

#[test]
fn builtin_clone_out_rejects_null_pointer() {
    let value = unsafe { <BuiltinParticipantSample as DdsType>::clone_out(std::ptr::null()) };
    let err = value.expect_err("a null sample pointer must not decode");
    assert!(
        format!("{err}").contains("null"),
        "error should name the null pointer, got: {err}"
    );
}

#[test]
fn builtin_clone_out_rejects_misaligned_pointer() {
    let buf = [0u8; 64];
    let value = unsafe {
        let misaligned = buf.as_ptr().add(1) as *const BuiltinTopicSample;
        <BuiltinTopicSample as DdsType>::clone_out(misaligned)
    };
    let err = value.expect_err("a misaligned sample pointer must not decode");
    assert!(
        format!("{err}").contains("unaligned"),
        "error should name the misalignment, got: {err}"
    );
}

#[cfg(feature = "serde")]
mod serde_guard {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Payload {
        id: u32,
    }

    serde_type_name!(Payload, "cyclonedds_test::InvalidPointerPayload");

    #[test]
    fn serde_clone_out_rejects_null_pointer() {
        // SAFETY: deliberately invalid pointer; see the file-level note.
        let value = unsafe { <SerdeSample<Payload> as DdsType>::clone_out(std::ptr::null()) };
        let err = value.expect_err("a null sample pointer must not decode");
        assert!(
            format!("{err}").contains("null"),
            "error should name the null pointer, got: {err}"
        );
    }
}
