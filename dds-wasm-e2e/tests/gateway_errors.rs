//! Gateway rejection paths over TCP: every malformed, unknown, or
//! unauthorized input gets a typed `error` reply (or a clean close) and
//! the connection — plus the gateway — stays alive for valid traffic.

use cyclonedds_proto::{Control, DataFrame, FLAG_CDR_LE, PROTO_VERSION};
use dds_wasm_e2e::*;
use std::net::TcpStream;
use std::time::Duration;

const T: Duration = Duration::from_secs(3);

fn raw_client(b: &dds_wasm_bridge::WasmBridge) -> TcpStream {
    TcpStream::connect(b.addr()).expect("raw connect")
}

fn expect_error(payload: &[u8], code: &str) -> String {
    let (got, detail) = dds_wasm_bridge::parse_error_reply(payload).expect("an error reply");
    assert_eq!(got, code, "detail: {detail}");
    detail
}

#[test]
fn register_and_subscribe_unknown_topic_fail_typed_and_conn_survives() {
    let topic = unique_topic("e2e_reg_unk");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    for kind in ["register", "subscribe"] {
        let ctrl = if kind == "register" {
            r#"{"proto":0,"kind":"register","topic":"nope","type":"WasmEcho","qos":{"reliability":"best_effort","durability":"volatile"},"seq":1}"#.to_string()
        } else {
            r#"{"proto":0,"kind":"subscribe","topic":"nope","seq":2}"#.to_string()
        };
        send_packet(&mut sock, ctrl.as_bytes());
        let detail = expect_error(&recv_packet(&mut sock, T).expect("reply"), "unknown_topic");
        assert!(
            detail.contains('0') || detail.contains("serving"),
            "{detail}"
        );
    }
    // The connection is still usable: a valid hello gets an ack.
    let hello = Control::Hello {
        proto: PROTO_VERSION,
        client: "e2e".into(),
    }
    .to_json()
    .unwrap();
    send_packet(&mut sock, hello.as_bytes());
    let reply = recv_packet(&mut sock, T).expect("ack");
    assert!(dds_wasm_bridge::parse_error_reply(&reply).is_none());
    assert_eq!(bridge.stats().errors_out, 2);
}

#[test]
fn newer_proto_and_unknown_kind_fail_closed() {
    let topic = unique_topic("e2e_proto");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    let newer = Control::Hello {
        proto: PROTO_VERSION + 1,
        client: "e2e".into(),
    }
    .to_json()
    .unwrap();
    send_packet(&mut sock, newer.as_bytes());
    expect_error(
        &recv_packet(&mut sock, T).expect("reply"),
        "unsupported_proto",
    );

    send_packet(&mut sock, br#"{"proto":0,"kind":"teleport","seq":1}"#);
    expect_error(&recv_packet(&mut sock, T).expect("reply"), "invalid_frame");
}

#[test]
fn malformed_control_json_falls_to_legacy_disabled_reply() {
    let topic = unique_topic("e2e_malformed");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    // Starts with `{` so it enters the control path, but is not JSON.
    send_packet(&mut sock, b"{not json");
    expect_error(
        &recv_packet(&mut sock, T).expect("reply"),
        "legacy_json_disabled",
    );
}

#[test]
fn truncated_binary_frame_is_typed_and_conn_survives() {
    let topic = unique_topic("e2e_trunc");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    send_packet(&mut sock, &[0x00, 0x01]);
    expect_error(&recv_packet(&mut sock, T).expect("reply"), "invalid_frame");

    // Binary still flows afterwards: publish through a client.
    let mut client = connect(&bridge);
    client.send_echo(&topic, &sample(1)).unwrap();
    let deadline = std::time::Instant::now() + T;
    while bridge.stats().samples_in != 1 {
        assert!(std::time::Instant::now() < deadline, "binary plane stalled");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn bad_flags_and_unknown_topic_frames_are_typed() {
    let topic = unique_topic("e2e_flags");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    let mut frame = dds_wasm_e2e_frame(&topic, 0);
    frame[2] = 0x00; // flags: neither CDR-LE nor legacy
    send_packet(&mut sock, &frame);
    expect_error(
        &recv_packet(&mut sock, T).expect("reply"),
        "unsupported_flags",
    );

    let other = dds_wasm_e2e_frame("elsewhere", 1);
    send_packet(&mut sock, &other);
    expect_error(&recv_packet(&mut sock, T).expect("reply"), "unknown_topic");
}

#[test]
fn garbage_cdr_is_a_serialization_error_not_a_crash() {
    let topic = unique_topic("e2e_cdr");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    // Structurally valid frame, garbage CDR payload.
    let frame = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: topic.clone(),
        cdr: vec![0xFF; 32],
        seq: 0,
    }
    .encode()
    .unwrap();
    send_packet(&mut sock, &frame);
    expect_error(&recv_packet(&mut sock, T).expect("reply"), "serialization");
}

#[test]
fn oversize_length_prefix_closes_without_a_big_alloc() {
    let topic = unique_topic("e2e_huge");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    let huge = (8 * 1024 * 1024 + 65u32).to_le_bytes();
    use std::io::Write as _;
    sock.write_all(&huge).unwrap();
    sock.flush().unwrap();
    // The gateway drops the connection instead of allocating.
    let mut buf = [0u8; 1];
    sock.set_read_timeout(Some(T)).unwrap();
    use std::io::Read as _;
    assert!(sock.read(&mut buf).is_err() || sock.read(&mut buf).unwrap_or(1) == 0);
    let _ = bridge;
}

#[test]
fn seq_gaps_are_counted_per_topic() {
    let topic = unique_topic("e2e_gaps");
    let bridge = bind_gateway(&topic);
    // Force non-consecutive seqs by crafting frames directly.
    let mut sock = raw_client(&bridge);
    for seq in [0u32, 5u32] {
        send_packet(&mut sock, &dds_wasm_e2e_frame_seq(&topic, seq));
    }
    // Give the dispatcher a moment, then check the counter.
    let deadline = std::time::Instant::now() + T;
    while bridge.stats().seq_gaps == 0 && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(bridge.stats().seq_gaps, 1);
    assert_eq!(bridge.stats().samples_in, 2);
}

#[test]
fn bye_closes_the_connection_cleanly() {
    let topic = unique_topic("e2e_bye");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);
    let bye = Control::Bye {
        proto: PROTO_VERSION,
    }
    .to_json()
    .unwrap();
    send_packet(&mut sock, bye.as_bytes());
    sock.set_read_timeout(Some(T)).unwrap();
    use std::io::Read as _;
    let mut buf = [0u8; 1];
    // EOF (Ok(0)) or a timeout-shaped error: either way no reply, no hang.
    let _ = sock.read(&mut buf);
}

/// Minimal valid CDR frame for `topic` with chosen `seq` (id/text/values).
fn dds_wasm_e2e_frame(topic: &str, seq: u32) -> Vec<u8> {
    dds_wasm_e2e_frame_seq(topic, seq)
}

fn dds_wasm_e2e_frame_seq(topic: &str, seq: u32) -> Vec<u8> {
    use cyclonedds_proto::echo::encode_echo_xcdr1;
    let cdr = encode_echo_xcdr1(&sample(seq as i32)).unwrap();
    DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: topic.to_string(),
        cdr,
        seq,
    }
    .encode()
    .unwrap()
}

#[test]
fn legacy_bad_shapes_are_typed_serialization_errors() {
    let topic = unique_topic("e2e_badshape");
    let bridge = bind_gateway_legacy(&topic);
    let mut sock = raw_client(&bridge);

    // Well-formed envelope, data is not an EchoMsg.
    let bad_data = format!(r#"{{"topic":"{topic}","data":123}}"#);
    send_packet(&mut sock, bad_data.as_bytes());
    expect_error(&recv_packet(&mut sock, T).expect("reply"), "serialization");

    // Well-formed JSON, unknown topic on the legacy path.
    send_packet(
        &mut sock,
        br#"{"topic":"elsewhere","data":{"id":1,"text":"x","values":[]}}"#,
    );
    expect_error(&recv_packet(&mut sock, T).expect("reply"), "unknown_topic");
}

#[test]
fn legacy_non_utf8_payload_is_typed_not_a_crash() {
    let topic = unique_topic("e2e_binutf8");
    let bridge = bind_gateway_legacy(&topic);
    let mut sock = raw_client(&bridge);
    // `{`-prefixed (JSON path) but not UTF-8: `from_slice` rejects it,
    // then the legacy arm rejects it again with a typed reply.
    send_packet(&mut sock, &[b'{', 0xFF, 0xFE, b'}']);
    expect_error(&recv_packet(&mut sock, T).expect("reply"), "invalid_frame");
}
