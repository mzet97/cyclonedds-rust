//! An unknown union discriminator must be an error, not a panic.
//!
//! `clone_out` for a `#[derive(DdsUnionDerive)]` type without a `#[dds_default]`
//! variant used to `panic!` when the discriminator was not a declared case. That
//! discriminator arrives from the network, so it is remote input: a peer built
//! from a different revision of the IDL was enough to make `reader.take()`
//! unwind on the caller's thread. The `catch_unwind` barriers kept it from
//! aborting the process through an `extern "C"` frame, but they could not stop
//! the unwind on the user's own thread.
//!
//! `clone_out` is fallible now, so an undecodable sample is reported and
//! discarded instead.

use cyclonedds::*;

#[repr(C)]
#[derive(Debug, Clone, DdsUnionDerive)]
#[dds_discriminant(u32)]
enum Choice {
    #[dds_case(1)]
    AsI32(i32),
    #[dds_case(2)]
    AsF64(f64),
}

/// A union carrying a declared discriminator decodes normally.
#[test]
fn declared_discriminator_decodes() {
    let native_size = <Choice as DdsType>::descriptor_size() as usize;
    let mut buf = vec![0u8; native_size.max(std::mem::size_of::<u32>() * 4)];

    // Discriminator 1 => AsI32.
    buf[..4].copy_from_slice(&1u32.to_ne_bytes());
    let value = unsafe { <Choice as DdsType>::clone_out(buf.as_ptr() as *const Choice) };
    assert!(
        matches!(value, Ok(Choice::AsI32(_))),
        "expected AsI32, got {value:?}"
    );
}

/// The case this phase exists for: a discriminator outside the declared set is
/// an `Err`, and getting there does not unwind.
#[test]
fn unknown_discriminator_is_an_error_not_a_panic() {
    let native_size = <Choice as DdsType>::descriptor_size() as usize;
    let mut buf = vec![0u8; native_size.max(std::mem::size_of::<u32>() * 4)];

    // 99 is not a declared case.
    buf[..4].copy_from_slice(&99u32.to_ne_bytes());

    // Deliberately *not* wrapped in catch_unwind: if this panics, the test
    // fails by unwinding, which is exactly the regression being guarded.
    let value = unsafe { <Choice as DdsType>::clone_out(buf.as_ptr() as *const Choice) };

    let err = value.expect_err("an undeclared discriminator must not decode");
    let msg = format!("{err}");
    assert!(
        msg.contains("discriminator") && msg.contains("99"),
        "error should name the offending discriminator, got: {msg}"
    );
}

/// Several unknown discriminators in a row must all report cleanly — the error
/// path must not leave the type in a state that breaks the next call.
#[test]
fn repeated_unknown_discriminators_stay_recoverable() {
    let native_size = <Choice as DdsType>::descriptor_size() as usize;
    for disc in [7u32, 42, 1000, u32::MAX] {
        let mut buf = vec![0u8; native_size.max(std::mem::size_of::<u32>() * 4)];
        buf[..4].copy_from_slice(&disc.to_ne_bytes());
        let value = unsafe { <Choice as DdsType>::clone_out(buf.as_ptr() as *const Choice) };
        assert!(value.is_err(), "discriminator {disc} should not decode");
    }

    // And a valid one still works afterwards.
    let mut buf = vec![0u8; native_size.max(std::mem::size_of::<u32>() * 4)];
    buf[..4].copy_from_slice(&2u32.to_ne_bytes());
    let value = unsafe { <Choice as DdsType>::clone_out(buf.as_ptr() as *const Choice) };
    assert!(matches!(value, Ok(Choice::AsF64(_))));
}
