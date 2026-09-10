//! Adversarial dispatcher paths: control closes/absorbs, newer binary
//! protos are refused, and oversize legacy payloads are skipped — the
//! gateway and the connection survive every one of them.

use cyclonedds_proto::echo::{encode_echo_xcdr1, EchoMsg};
use cyclonedds_proto::{Control, DataFrame, FLAG_CDR_LE, MAX_FRAME_BYTES, PROTO_VERSION};
use dds_wasm_e2e::*;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

fn raw_client(b: &dds_wasm_bridge::WasmBridge) -> TcpStream {
    TcpStream::connect(b.addr()).expect("raw connect")
}

fn expect_error(payload: &[u8], code: &str) {
    let (got, detail) = dds_wasm_bridge::parse_error_reply(payload).expect("an error reply");
    assert_eq!(got, code, "detail: {detail}");
}

fn wait_for_stats(
    b: &dds_wasm_bridge::WasmBridge,
    f: impl Fn(dds_wasm_bridge::BridgeStats) -> bool,
) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while !f(b.stats()) {
        assert!(std::time::Instant::now() < deadline, "data plane stalled");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn bye_control_closes_the_connection_while_gateway_survives() {
    // Given a live connection.
    let topic = unique_topic("e2e_bye");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    // When the client says bye, then the server side tears the connection
    // down: reader unregisters, the pump's queue sender drops, the pump
    // exits, and the last socket half closes (EOF, not a hang).
    let bye = Control::Bye {
        proto: PROTO_VERSION,
    }
    .to_json()
    .unwrap();
    send_packet(&mut sock, bye.as_bytes());
    sock.set_read_timeout(Some(T)).unwrap();
    let mut buf = [0u8; 1];
    assert_eq!(sock.read(&mut buf).unwrap(), 0, "bye must close, not hang");

    // And the gateway itself is unaffected: a new client gets an ack.
    let mut sock2 = raw_client(&bridge);
    let hello = Control::Hello {
        proto: PROTO_VERSION,
        client: "e2e".into(),
    }
    .to_json()
    .unwrap();
    send_packet(&mut sock2, hello.as_bytes());
    let reply = recv_packet(&mut sock2, T).expect("ack after bye");
    assert!(dds_wasm_bridge::parse_error_reply(&reply).is_none());
}

#[test]
fn ack_control_is_absorbed_without_reply_and_conn_survives() {
    // Given a live connection.
    let topic = unique_topic("e2e_ack");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    // When a stray ack arrives, then it is absorbed silently (no reply)
    // and the connection stays usable.
    let ack = Control::Ack {
        proto: PROTO_VERSION,
        seq: 7,
    }
    .to_json()
    .unwrap();
    send_packet(&mut sock, ack.as_bytes());
    assert!(
        recv_packet(&mut sock, Duration::from_millis(400)).is_none(),
        "ack must not be answered"
    );
    let hello = Control::Hello {
        proto: PROTO_VERSION,
        client: "e2e".into(),
    }
    .to_json()
    .unwrap();
    send_packet(&mut sock, hello.as_bytes());
    let reply = recv_packet(&mut sock, T).expect("ack");
    assert!(dds_wasm_bridge::parse_error_reply(&reply).is_none());
}

#[test]
fn newer_binary_proto_is_unsupported_but_conn_survives() {
    // Given a live connection serving `topic`.
    let topic = unique_topic("e2e_binproto");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    // When a structurally valid frame carries a newer proto version, then
    // it is refused with a typed reply (decode parses everything first,
    // the version check fires last).
    let cdr = encode_echo_xcdr1(&sample(1)).unwrap();
    let frame = DataFrame {
        proto: PROTO_VERSION + 1,
        flags: FLAG_CDR_LE,
        topic: topic.clone(),
        cdr,
        seq: 0,
    }
    .encode()
    .unwrap();
    send_packet(&mut sock, &frame);
    expect_error(
        &recv_packet(&mut sock, T).expect("reply"),
        "unsupported_proto",
    );

    // And the connection still serves current-proto traffic.
    let cdr = encode_echo_xcdr1(&sample(2)).unwrap();
    let frame = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: topic.clone(),
        cdr,
        seq: 1,
    }
    .encode()
    .unwrap();
    send_packet(&mut sock, &frame);
    wait_for_stats(&bridge, |s| s.samples_in == 1);
    assert_eq!(bridge.stats().errors_out, 1);
}

#[test]
fn declared_huge_topic_len_is_too_large_without_allocating() {
    // Given a live connection.
    let topic = unique_topic("e2e_toolarge");
    let bridge = bind_gateway(&topic);
    let mut sock = raw_client(&bridge);

    // When a frame declares a topic_len past the frame cap, then decode
    // refuses before slicing or allocating (the length prefix itself is
    // 20 bytes) and the connection survives for valid traffic.
    let mut payload = Vec::new();
    payload.extend_from_slice(&PROTO_VERSION.to_le_bytes());
    payload.extend_from_slice(&FLAG_CDR_LE.to_le_bytes());
    payload.extend_from_slice(&((MAX_FRAME_BYTES as u32) + 1).to_le_bytes());
    payload.extend_from_slice(&[0u8; 8]);
    send_packet(&mut sock, &payload);
    expect_error(
        &recv_packet(&mut sock, T).expect("reply"),
        "frame_too_large",
    );

    let cdr = encode_echo_xcdr1(&sample(1)).unwrap();
    let frame = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: topic.clone(),
        cdr,
        seq: 0,
    }
    .encode()
    .unwrap();
    send_packet(&mut sock, &frame);
    wait_for_stats(&bridge, |s| s.samples_in == 1);
    assert_eq!(bridge.stats().errors_out, 1);
}

#[test]
fn legacy_oversize_values_are_skipped_and_gateway_survives() {
    // Given a legacy-compat gateway: the binary decoder caps `values` at
    // 262144 elements, but legacy JSON has no such cap, so a 300000-long
    // sequence reaches DDS, comes back, and fails the re-encode.
    let topic = unique_topic("e2e_bigleg");
    let bridge = bind_gateway_legacy(&topic);
    let mut sock = raw_client(&bridge);

    // When the oversize envelope arrives, then it is published (channel
    // send works) but skipped on the way back (encode TooLarge): no
    // crash, no reply, no stuck dispatcher.
    let vals = "7,".repeat(300_000);
    let env = format!(
        "{{\"topic\":{topic:?},\"data\":{{\"id\":1,\"text\":\"big\",\"values\":[{vals}0]}}}}"
    );
    send_packet(&mut sock, env.as_bytes());
    wait_for_stats(&bridge, |s| s.samples_in == 1);
    assert!(
        recv_packet(&mut sock, Duration::from_millis(500)).is_none(),
        "oversize sample must be skipped, not routed"
    );

    // And a normal sample right after still round-trips end to end.
    let small =
        format!("{{\"topic\":{topic:?},\"data\":{{\"id\":2,\"text\":\"ok\",\"values\":[1]}}}}");
    send_packet(&mut sock, small.as_bytes());
    wait_for_stats(&bridge, |s| s.samples_in == 2);
    let routed = recv_packet(&mut sock, T).expect("routed frame");
    let back = DataFrame::decode(&routed).expect("valid frame");
    assert_eq!(back.topic, topic);
    let msg: EchoMsg = cyclonedds_proto::echo::decode_echo_xcdr1(&back.cdr).unwrap();
    assert_eq!(msg.id, 2);
}
