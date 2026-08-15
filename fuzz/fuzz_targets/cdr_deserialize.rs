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

// Fuzz target: feed arbitrary bytes to CdrDeserializer.
//
// Both XCDR1 and XCDR2. The deserializer should never panic or abort — it may
// return Err(..) for invalid input, but must not crash. It did: see
// `cyclonedds-test-suite/tests/cdr_deserialize_corpus.rs`, which runs this same
// property with a seeded PRNG so it also executes where libFuzzer cannot.
//
// (A doc comment here is a warning: rustdoc does not document macro invocations.)
fuzz_target!(|data: &[u8]| {
    let _ = CdrDeserializer::<FuzzSample>::deserialize(data, CdrEncoding::Xcdr1);
    let _ = CdrDeserializer::<FuzzSample>::deserialize(data, CdrEncoding::Xcdr2);
});
