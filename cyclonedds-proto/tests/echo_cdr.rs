use cyclonedds_proto::echo::*;
use cyclonedds_proto::{ProtoError, FLAG_CDR_LE, PROTO_VERSION};

fn msg() -> EchoMsg {
    EchoMsg {
        id: 1,
        text: "hi".into(),
        values: vec![1, 2, 3],
    }
}

#[test]
fn echo_roundtrip() {
    let m = msg();
    let bytes = encode_echo_xcdr1(&m).unwrap();
    // id=1 | strlen=3 | "hi\0" + pad | seqlen=3 | 1,2,3
    assert_eq!(
        bytes,
        vec![
            1, 0, 0, 0, 3, 0, 0, 0, b'h', b'i', 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0,
            0
        ]
    );
    assert_eq!(decode_echo_xcdr1(&bytes).unwrap(), m);
}

#[test]
fn echo_empty_string_and_seq() {
    let m = EchoMsg {
        id: -7,
        text: String::new(),
        values: vec![],
    };
    let bytes = encode_echo_xcdr1(&m).unwrap();
    // id | strlen=1 | NUL + 3 pad | seqlen=0
    assert_eq!(bytes.len(), 16);
    assert_eq!(decode_echo_xcdr1(&bytes).unwrap(), m);
}

#[test]
fn echo_unaligned_string_padding() {
    // text_len incl NUL = 5 -> 3 pad bytes; then seq must still align.
    let m = EchoMsg {
        id: 9,
        text: "abcd".into(),
        values: vec![-1, i32::MIN, i32::MAX],
    };
    let bytes = encode_echo_xcdr1(&m).unwrap();
    assert_eq!(decode_echo_xcdr1(&bytes).unwrap(), m);
    // offsets: id[0..4] len[4..8] str[8..13] pad[13..16] seqlen[16..20]
    assert_eq!(&bytes[16..20], &[3, 0, 0, 0]);
}

#[test]
fn echo_truncated_inputs_are_typed_errors() {
    let bytes = encode_echo_xcdr1(&msg()).unwrap();
    for cut in [0, 1, 5, 8, 11, 12, 15, 16, 19, 20, 27] {
        let err = decode_echo_xcdr1(&bytes[..cut]).unwrap_err();
        assert!(
            matches!(err, ProtoError::InvalidFrame(_)),
            "cut {cut}: {err:?}"
        );
    }
    // trailing byte rejected
    let mut long = bytes.clone();
    long.push(0);
    assert!(matches!(
        decode_echo_xcdr1(&long).unwrap_err(),
        ProtoError::InvalidFrame(_)
    ));
}

#[test]
fn echo_adversarial_lengths_never_panic_nor_allocate_big() {
    // huge string length, tiny buffer
    let evil_str = [0u8, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0x7F];
    assert!(matches!(
        decode_echo_xcdr1(&evil_str).unwrap_err(),
        ProtoError::TooLarge { .. }
    ));
    // zero-length string
    let zero_str = [5u8, 0, 0, 0, 0, 0, 0, 0];
    assert!(matches!(
        decode_echo_xcdr1(&zero_str).unwrap_err(),
        ProtoError::InvalidFrame(_)
    ));
    // missing NUL terminator
    let no_nul = [5u8, 0, 0, 0, 3, 0, 0, 0, b'a', b'b', b'c', 0, 0, 0, 0, 0];
    assert!(decode_echo_xcdr1(&no_nul).is_err());
    // invalid UTF-8
    let bad_utf8 = [5u8, 0, 0, 0, 2, 0, 0, 0, 0xFF, 0];
    assert!(matches!(
        decode_echo_xcdr1(&bad_utf8).unwrap_err(),
        ProtoError::Serialization(_)
    ));
    // huge sequence length, tiny buffer
    let mut evil_seq = encode_echo_xcdr1(&EchoMsg {
        id: 1,
        text: String::new(),
        values: vec![],
    })
    .unwrap();
    evil_seq[12..16].copy_from_slice(&0xFF_FF_FF_7Fu32.to_le_bytes());
    assert!(matches!(
        decode_echo_xcdr1(&evil_seq).unwrap_err(),
        ProtoError::TooLarge { .. }
    ));
}

#[test]
fn echo_frame_constants_agree_with_proto() {
    assert_eq!(PROTO_VERSION, 0);
    assert_eq!(FLAG_CDR_LE, 0x01);
    assert_eq!(ECHO_TOPIC, "WasmEcho");
}

#[test]
fn echo_encode_rejects_oversize_text_with_sizes() {
    let m = EchoMsg {
        id: 1,
        text: "x".repeat(MAX_ECHO_TEXT + 1),
        values: vec![],
    };
    assert!(matches!(
        encode_echo_xcdr1(&m).unwrap_err(),
        ProtoError::TooLarge { got, max }
            if got == MAX_ECHO_TEXT + 1 && max == MAX_ECHO_TEXT
    ));
}

#[test]
fn echo_encode_rejects_oversize_values_with_sizes() {
    let m = EchoMsg {
        id: 1,
        text: "ok".into(),
        values: vec![0; MAX_ECHO_VALUES + 1],
    };
    assert!(matches!(
        encode_echo_xcdr1(&m).unwrap_err(),
        ProtoError::TooLarge { got, max }
            if got == (MAX_ECHO_VALUES + 1) * 4 && max == MAX_ECHO_VALUES * 4
    ));
}

#[test]
fn echo_encode_rejects_interior_nul() {
    let m = EchoMsg {
        id: 1,
        text: "a\0b".into(),
        values: vec![],
    };
    let err = encode_echo_xcdr1(&m).unwrap_err().to_string();
    assert!(err.contains("interior NUL"), "unexpected: {err}");
}
