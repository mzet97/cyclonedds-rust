#![no_main]

use cyclonedds::{CdrDeserializer, CdrEncoding, DdsTypeDerive};
use libfuzzer_sys::fuzz_target;

/// A simple struct used as the fuzzing target for CDR deserialization.
#[repr(C)]
#[derive(DdsTypeDerive)]
struct FuzzSample {
    id: i32,
    payload: [u8; 64],
}

/// Fuzz target: feed arbitrary bytes to CdrDeserializer.
///
/// We test both XCDR1 and XCDR2 encodings.  The deserializer should never
/// panic or abort — it may return Err(...) for invalid input, but must not
/// crash.
fuzz_target!(|data: &[u8]| {
    let _ = CdrDeserializer::<FuzzSample>::deserialize(data, CdrEncoding::Xcdr1);
    let _ = CdrDeserializer::<FuzzSample>::deserialize(data, CdrEncoding::Xcdr2);
});
