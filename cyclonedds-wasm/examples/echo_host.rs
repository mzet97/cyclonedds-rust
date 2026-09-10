//! Host-runnable example (Phase H): exercises the consumable package
//! surface without a browser — portable codec roundtrip, gateway QoS
//! treatment, BigInt decimal mapping, and the typed host transport gap.
//!
//! Run: `cargo run -p cyclonedds-wasm --example echo_host`

use cyclonedds_proto::echo::{EchoMsg, ECHO_TOPIC};
use cyclonedds_wasm::{
    check_gateway_qos, decimal_str_to_i64, decimal_str_to_u64, decode_echo_frame,
    encode_echo_frame, i64_to_decimal_str, u64_needs_bigint, u64_to_decimal_str,
    MAX_SAFE_INTEGER_U64,
};

fn main() {
    // 1. Binary data-plane roundtrip (same bytes shipped in the wasm pkg).
    let msg = EchoMsg {
        id: 7,
        text: "hello from host example".into(),
        values: vec![1, 2, 3],
    };
    let frame = encode_echo_frame(ECHO_TOPIC, &msg, 0).expect("encode");
    let (topic, back, seq) = decode_echo_frame(&frame).expect("decode");
    assert_eq!(topic, ECHO_TOPIC);
    assert_eq!(back, msg);
    assert_eq!(seq, 0);
    println!("echo frame roundtrip: OK ({} bytes)", frame.len());

    // 2. Reliable QoS is rejected with Unsupported (treatment, not impl).
    let reliable = cyclonedds_proto::Qos {
        reliability: cyclonedds_proto::Reliability::Reliable,
        durability: cyclonedds_proto::Durability::Volatile,
    };
    assert!(check_gateway_qos(&reliable).is_err());
    println!("reliable -> Unsupported: OK");

    // 3. u64/i64 cross as BigInt: exact decimal mapping, incl. > 2^53.
    assert_eq!(u64_to_decimal_str(u64::MAX), "18446744073709551615");
    assert_eq!(
        decimal_str_to_u64("9007199254740993").unwrap(),
        MAX_SAFE_INTEGER_U64 + 2
    );
    assert!(u64_needs_bigint(MAX_SAFE_INTEGER_U64 + 1));
    assert!(!u64_needs_bigint(MAX_SAFE_INTEGER_U64));
    assert_eq!(i64_to_decimal_str(i64::MIN), "-9223372036854775808");
    assert_eq!(
        decimal_str_to_i64("-9223372036854775808").unwrap(),
        i64::MIN
    );
    println!("bigint u64/i64 mapping: OK (exact above 2^53)");

    // 4. Host has no socket: write path validates framing, then reports
    // the transport gap as typed NotConnected (never a panic, never a
    // silent drop).
    let participant =
        cyclonedds_wasm::WasmDomainParticipant::new("ws://127.0.0.1:9/dds").expect("participant");
    let topic = participant
        .create_topic::<EchoMsg>(ECHO_TOPIC)
        .expect("topic");
    let writer = participant.create_echo_writer(&topic).expect("writer");
    match writer.write_echo(&msg) {
        Err(cyclonedds_wasm::WasmDdsError::NotConnected) => {
            println!("host write -> NotConnected: OK (transport gap is typed)")
        }
        other => panic!(
            "expected NotConnected, got {:?}",
            std::mem::discriminant(&other)
        ),
    }
}
