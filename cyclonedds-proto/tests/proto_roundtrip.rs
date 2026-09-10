//! Portable contract tests: golden fixtures + negative (Unsupported /
//! never-panic) treatment proofs. These run on every target, including
//! wasm32, because `cyclonedds-proto` has no native dependency.

use cyclonedds_proto::*;
use serde_json::{json, Value};

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read_to_string(path).unwrap()
}

fn canonical(v: &str) -> Value {
    serde_json::from_str(v).unwrap()
}

#[test]
fn hello_fixture_roundtrips() {
    let raw = fixture("hello.json");
    let msg = Control::from_json(&raw).unwrap();
    assert_eq!(
        msg,
        Control::Hello {
            proto: 0,
            client: "wasm-consumer-min/0.1".into()
        }
    );
    msg.check_version().unwrap();
    assert_eq!(canonical(&msg.to_json().unwrap()), canonical(&raw));
}

#[test]
fn register_fixture_carries_qos_and_seq() {
    let raw = fixture("register.json");
    let msg = Control::from_json(&raw).unwrap();
    match &msg {
        Control::Register {
            topic,
            type_name,
            qos,
            seq,
            ..
        } => {
            assert_eq!(topic, "HelloWorld");
            assert_eq!(type_name, "MyMessage");
            assert_eq!(*qos, Qos::default());
            assert_eq!(*seq, 1);
            check_qos(qos).unwrap();
        }
        other => panic!("unexpected control: {other:?}"),
    }
    assert_eq!(canonical(&msg.to_json().unwrap()), canonical(&raw));
}

#[test]
fn error_fixture_is_treatment_not_crash() {
    let raw = fixture("error_unsupported_qos.json");
    let msg = Control::from_json(&raw).unwrap();
    match msg {
        Control::Error { code, .. } => assert_eq!(code, "unsupported_qos"),
        other => panic!("unexpected control: {other:?}"),
    }
}

#[test]
fn reliable_qos_is_unsupported_not_implemented() {
    // Negative proof of TREATMENT (REQ-QOS-01): requesting reliable must
    // yield Unsupported; this test passing does NOT mean reliable works.
    let qos = Qos {
        reliability: Reliability::Reliable,
        durability: Durability::Volatile,
    };
    assert_eq!(
        check_qos(&qos),
        Err(ProtoError::Unsupported(
            "reliability != best_effort".to_string()
        ))
    );
    let qos = Qos {
        reliability: Reliability::BestEffort,
        durability: Durability::TransientLocal,
    };
    assert!(matches!(check_qos(&qos), Err(ProtoError::Unsupported(_))));
}

#[test]
fn newer_proto_is_unsupported() {
    let msg = Control::Hello {
        proto: 99,
        client: "future".into(),
    };
    assert!(matches!(
        msg.check_version(),
        Err(ProtoError::Unsupported(_))
    ));
}

#[test]
fn legacy_envelope_fixture_decodes() {
    let raw = fixture("legacy_envelope.json");
    let env = LegacyEnvelope::decode(raw.trim()).unwrap();
    assert_eq!(env.topic, "HelloWorld");
    assert_eq!(env.data, json!({"id": 1, "text": "hello"}));
    // Re-encode is byte-stable through canonical JSON comparison.
    assert_eq!(
        canonical(&LegacyEnvelope::encode(&env.topic, &env.data).unwrap()),
        canonical(&raw)
    );
}

#[test]
fn data_frame_roundtrips_byte_identical() {
    let frame = DataFrame {
        proto: 0,
        flags: FLAG_CDR_LE,
        topic: "HelloWorld".into(),
        cdr: vec![1, 2, 3, 4],
        seq: 7,
    };
    let bytes = frame.encode().unwrap();
    // Header layout: u16 proto | u16 flags | u32 topic_len | topic |
    // u32 cdr_len | cdr | u32 seq (all LE).
    assert_eq!(&bytes[0..2], &0u16.to_le_bytes());
    assert_eq!(&bytes[2..4], &FLAG_CDR_LE.to_le_bytes());
    assert_eq!(&bytes[4..8], &10u32.to_le_bytes());
    assert_eq!(&bytes[18..22], &4u32.to_le_bytes());
    assert_eq!(&bytes[bytes.len() - 4..], &7u32.to_le_bytes());
    assert_eq!(DataFrame::decode(&bytes).unwrap(), frame);
}

#[test]
fn truncated_and_adversarial_frames_never_panic() {
    let good = DataFrame {
        proto: 0,
        flags: FLAG_CDR_LE,
        topic: "T".into(),
        cdr: vec![0xAA],
        seq: 1,
    }
    .encode()
    .unwrap();
    // Every prefix of a valid frame must be a typed error, never a panic.
    for len in 0..good.len() {
        let r = DataFrame::decode(&good[..len]);
        assert!(r.is_err(), "prefix len {len} unexpectedly decoded");
    }
    // Mutually exclusive flags.
    let mut bad = good.clone();
    bad[2] = (FLAG_CDR_LE | FLAG_LEGACY_JSON) as u8;
    assert!(matches!(
        DataFrame::decode(&bad),
        Err(ProtoError::InvalidFrame(_))
    ));
    // Non-UTF8 topic.
    let mut bad = good.clone();
    bad[8] = 0xFF;
    assert!(matches!(
        DataFrame::decode(&bad),
        Err(ProtoError::InvalidFrame(_))
    ));
    // Trailing garbage.
    let mut bad = good.clone();
    bad.extend_from_slice(&[0u8; 3]);
    assert!(matches!(
        DataFrame::decode(&bad),
        Err(ProtoError::InvalidFrame(_))
    ));
    // Empty input.
    assert!(matches!(
        DataFrame::decode(&[]),
        Err(ProtoError::InvalidFrame(_))
    ));
}

#[test]
fn proto_error_display_names_every_variant() {
    assert_eq!(
        ProtoError::Unsupported("q".into()).to_string(),
        "unsupported: q"
    );
    assert_eq!(
        ProtoError::Serialization("s".into()).to_string(),
        "serialization error: s"
    );
    assert_eq!(
        ProtoError::TooLarge { got: 9, max: 8 }.to_string(),
        "frame too large: 9 bytes (max 8)"
    );
    assert_eq!(
        ProtoError::InvalidFrame("t").to_string(),
        "invalid frame: t"
    );
    assert_eq!(ProtoError::NotConnected.to_string(), "not connected");
}

#[test]
fn control_proto_reports_version_for_every_variant() {
    let qos = Qos::default();
    let cases = [
        Control::Hello {
            proto: 3,
            client: "c".into(),
        },
        Control::Register {
            proto: 3,
            topic: "t".into(),
            type_name: "T".into(),
            qos,
            seq: 0,
        },
        Control::Subscribe {
            proto: 3,
            topic: "t".into(),
            seq: 0,
        },
        Control::Unsubscribe {
            proto: 3,
            topic: "t".into(),
            seq: 0,
        },
        Control::Bye { proto: 3 },
        Control::Ack { proto: 3, seq: 0 },
        Control::Error {
            proto: 3,
            code: "c".into(),
            detail: "d".into(),
        },
    ];
    for c in cases {
        assert_eq!(c.proto(), 3);
    }
}

#[test]
fn control_check_version_rejects_newer_proto() {
    let newer = Control::Hello {
        proto: PROTO_VERSION + 1,
        client: "c".into(),
    };
    let err = newer.check_version().unwrap_err().to_string();
    assert!(err.contains("proto"), "unexpected: {err}");
    Control::Bye {
        proto: PROTO_VERSION,
    }
    .check_version()
    .unwrap();
}

#[test]
fn dataframe_encode_rejects_mixed_legacy_and_binary_flags() {
    let frame = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE | FLAG_LEGACY_JSON,
        topic: "t".into(),
        cdr: vec![1],
        seq: 0,
    };
    let err = frame.encode().unwrap_err().to_string();
    assert!(err.contains("mutually exclusive"), "unexpected: {err}");
}

#[test]
fn dataframe_encode_rejects_oversize_payload() {
    let frame = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: "t".into(),
        cdr: vec![0u8; MAX_FRAME_BYTES],
        seq: 0,
    };
    assert!(matches!(
        frame.encode().unwrap_err(),
        ProtoError::TooLarge { got, max } if got > max && max == MAX_FRAME_BYTES
    ));
}

#[test]
fn dataframe_decode_rejects_mixed_flags_oversize_and_newer_proto() {
    // Mixed flags on the wire.
    let mut bad_flags = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: "t".into(),
        cdr: vec![1],
        seq: 0,
    }
    .encode()
    .unwrap();
    bad_flags[2] |= FLAG_LEGACY_JSON as u8;
    assert!(DataFrame::decode(&bad_flags)
        .unwrap_err()
        .to_string()
        .contains("mutually exclusive"));

    // Declared topic length beyond the cap: TooLarge without a big alloc.
    // Layout: proto[0..2] flags[2..4] topic_len[4..8].
    let mut bad_topic = vec![0u8; 16];
    bad_topic[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        DataFrame::decode(&bad_topic).unwrap_err(),
        ProtoError::TooLarge { got, max }
            if got == u32::MAX as usize && max == MAX_FRAME_BYTES
    ));

    // Declared cdr length beyond the cap (valid topic, exact trailers).
    let mut bad_cdr = vec![0u8; 4 + 4 + 1 + 4 + 4];
    bad_cdr[0..2].copy_from_slice(&PROTO_VERSION.to_le_bytes());
    bad_cdr[2..4].copy_from_slice(&FLAG_CDR_LE.to_le_bytes());
    bad_cdr[4..8].copy_from_slice(&1u32.to_le_bytes());
    bad_cdr[8] = b't';
    bad_cdr[9..13].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(matches!(
        DataFrame::decode(&bad_cdr).unwrap_err(),
        ProtoError::TooLarge { got, max }
            if got == u32::MAX as usize && max == MAX_FRAME_BYTES
    ));

    // Newer protocol version decodes structurally, then fails closed.
    let mut newer = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: "t".into(),
        cdr: vec![1],
        seq: 0,
    }
    .encode()
    .unwrap();
    newer[0..2].copy_from_slice(&(PROTO_VERSION + 1).to_le_bytes());
    let err = DataFrame::decode(&newer).unwrap_err().to_string();
    assert!(err.contains("proto"), "unexpected: {err}");
}
