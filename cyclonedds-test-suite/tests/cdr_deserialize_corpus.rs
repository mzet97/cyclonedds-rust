//! The `fuzz/cdr_deserialize` property, run deterministically.
//!
//! `fuzz/fuzz_targets/cdr_deserialize.rs` asserts that `CdrDeserializer` never
//! panics on arbitrary bytes — it may return `Err`, it may not crash. That
//! target had never run: the crate was neither a workspace member nor excluded
//! and had no `[workspace]` table of its own, so every cargo command in `fuzz/`
//! failed before compiling anything. It builds now, but `cargo fuzz` still needs
//! libFuzzer, which rules out `x86_64-pc-windows-msvc` and therefore most of
//! this project's local development.
//!
//! So the same property is asserted here with a seeded PRNG instead of a
//! coverage-guided one. That is strictly weaker at finding inputs, and strictly
//! stronger at *staying* run: it needs no toolchain beyond the MSRV, it is
//! deterministic, and it executes on every CI platform. The libFuzzer target is
//! kept for the platforms that can host it.
//!
//! Under AddressSanitizer this also covers the read side of the deserializer,
//! which is where a bad length prefix would show up as an overread rather than
//! as a panic.

use cyclonedds::{CdrDeserializer, CdrEncoding, CdrSerializer, DdsTypeDerive};

/// Mirrors the type in the libFuzzer target.
#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct FuzzSample {
    id: i32,
    payload: [u8; 64],
}

/// A type with heap-backed fields, where a bad length prefix has somewhere to go.
#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct HeapSample {
    #[key]
    id: i32,
    name: String,
    values: Vec<i32>,
}

/// xorshift64*, so the corpus is identical on every platform and every run.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn fill(&mut self, buf: &mut [u8]) {
        for chunk in buf.chunks_mut(8) {
            let bytes = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
        }
    }
}

fn both_encodings<T: cyclonedds::DdsType>(data: &[u8]) {
    // The contract is "no panic"; the result is deliberately discarded.
    let _ = CdrDeserializer::<T>::deserialize(data, CdrEncoding::Xcdr1);
    let _ = CdrDeserializer::<T>::deserialize(data, CdrEncoding::Xcdr2);
    let _ = CdrDeserializer::<T>::deserialize_key(data, CdrEncoding::Xcdr1);
    let _ = CdrDeserializer::<T>::deserialize_key(data, CdrEncoding::Xcdr2);
}

#[test]
fn random_bytes_never_panic_for_a_pod_type() {
    let mut rng = Rng(0x1234_5678_9ABC_DEF0);
    let mut buf = vec![0u8; 128];

    for _ in 0..2_000 {
        let len = (rng.next_u64() % 129) as usize;
        buf.resize(len, 0);
        rng.fill(&mut buf);
        both_encodings::<FuzzSample>(&buf);
    }
}

#[test]
fn random_bytes_never_panic_for_a_heap_type() {
    let mut rng = Rng(0x0FED_CBA9_8765_4321);
    let mut buf = vec![0u8; 128];

    for _ in 0..2_000 {
        let len = (rng.next_u64() % 129) as usize;
        buf.resize(len, 0);
        rng.fill(&mut buf);
        both_encodings::<HeapSample>(&buf);
    }
}

/// The shapes a random walk is unlikely to reach on its own.
#[test]
fn adversarial_inputs_never_panic() {
    let cases: Vec<Vec<u8>> = vec![
        // Nothing at all, and less than a CDR header.
        vec![],
        vec![0x00],
        vec![0x00, 0x01],
        vec![0x00, 0x01, 0x00],
        // A plausible XCDR1 big/little-endian header and then nothing.
        vec![0x00, 0x00, 0x00, 0x00],
        vec![0x00, 0x01, 0x00, 0x00],
        // Header plus a length prefix claiming far more than follows: the case
        // that turns into an overread rather than a panic.
        {
            let mut v = vec![0x00, 0x01, 0x00, 0x00];
            v.extend_from_slice(&u32::MAX.to_le_bytes());
            v
        },
        {
            let mut v = vec![0x00, 0x01, 0x00, 0x00];
            v.extend_from_slice(&0x7FFF_FFFFu32.to_le_bytes());
            v.extend_from_slice(b"short");
            v
        },
        // A string length with no NUL terminator anywhere.
        {
            let mut v = vec![0x00, 0x01, 0x00, 0x00];
            v.extend_from_slice(&4i32.to_le_bytes()); // id
            v.extend_from_slice(&64u32.to_le_bytes()); // string length
            v.extend_from_slice(&[b'A'; 8]); // ...but only 8 bytes of it
            v
        },
        // All bits set, at a few lengths.
        vec![0xFF; 4],
        vec![0xFF; 16],
        vec![0xFF; 256],
    ];

    for case in &cases {
        both_encodings::<FuzzSample>(case);
        both_encodings::<HeapSample>(case);
    }
}

/// The validation added alongside these tests must reject malformed input
/// without rejecting valid input.
///
/// Without this, "never panics" would be satisfied by refusing everything.
#[test]
fn well_formed_input_still_round_trips() {
    for encoding in [CdrEncoding::Xcdr1, CdrEncoding::Xcdr2] {
        let sample = HeapSample {
            id: 7,
            name: "round-trip".to_string(),
            values: vec![1, 2, 3],
        };

        let bytes = CdrSerializer::<HeapSample>::serialize(&sample, encoding)
            .unwrap_or_else(|e| panic!("serialize failed for {encoding:?}: {e}"));
        let back = CdrDeserializer::<HeapSample>::deserialize(&bytes, encoding)
            .unwrap_or_else(|e| panic!("deserialize rejected its own output for {encoding:?}: {e}"));

        assert_eq!(back.id, sample.id);
        assert_eq!(back.name, sample.name);
        assert_eq!(back.values, sample.values);

        let key_bytes = CdrSerializer::<HeapSample>::serialize_key(&sample, encoding)
            .unwrap_or_else(|e| panic!("serialize_key failed for {encoding:?}: {e}"));
        let key_back = CdrDeserializer::<HeapSample>::deserialize_key(&key_bytes, encoding)
            .unwrap_or_else(|e| {
                panic!("deserialize_key rejected its own output for {encoding:?}: {e}")
            });
        assert_eq!(key_back.id, sample.id);
    }
}

/// Truncating valid bytes must be rejected, not followed.
///
/// This is the shape that used to walk off the end: a length prefix that is
/// itself well-formed, describing more data than the buffer holds.
#[test]
fn truncated_valid_input_is_rejected() {
    let sample = HeapSample {
        id: 7,
        name: "a reasonably long name".to_string(),
        values: vec![1, 2, 3, 4, 5, 6, 7, 8],
    };
    let bytes = CdrSerializer::<HeapSample>::serialize(&sample, CdrEncoding::Xcdr1).unwrap();
    assert!(bytes.len() > 8, "need something to truncate");

    for cut in 1..bytes.len() {
        let truncated = &bytes[..cut];
        assert!(
            CdrDeserializer::<HeapSample>::deserialize(truncated, CdrEncoding::Xcdr1).is_err(),
            "a {cut}-byte prefix of a {}-byte sample was accepted",
            bytes.len()
        );
    }
}
