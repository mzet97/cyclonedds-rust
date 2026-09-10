//! Phase C first slice: browser -> binary CDR -> native gateway -> native
//! DDS peer, both directions, with real entities and real sockets.
//!
//! The "browser" is [`BridgeClient`], which emits the exact [`DataFrame`]
//! bytes the wasm32 `WasmEchoWriter` produces (same `cyclonedds-proto`
//! codec); only the socket type differs (TCP here, WebSocket in the
//! browser), so this exercises the real wire contract end to end.

use cyclonedds_proto::echo::EchoMsg;
use dds_wasm_bridge::{BridgeClient, BridgeConfig, WasmBridge, WasmEcho};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Sandbox-proofing: the machine default CycloneDDS interface selector may
/// name a NIC that does not exist here (observed: `enp4s0` vs live
/// `enp7s0`), which fails every `DomainParticipant::new` with
/// `ReturnCode(-1)`. Pin loopback once per process; loopback is all these
/// tests need. Same value for every test, applied under `Once`.
fn ensure_loopback_dds() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var(
            "CYCLONEDDS_URI",
            r#"<CycloneDDS><Domain><General><Interfaces><NetworkInterface name="lo"/></Interfaces></General></Domain></CycloneDDS>"#,
        );
    });
}

fn unique_topic(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}_{}_{}", std::process::id(), nanos)
}

fn sample() -> EchoMsg {
    EchoMsg {
        id: 7,
        text: "chave+string+sequência ✓".into(),
        values: vec![-1, 0, 1, 2, 3],
    }
}

struct Peer {
    _participant: cyclonedds::DomainParticipant,
    _topic: cyclonedds::Topic<WasmEcho>,
    writer: cyclonedds::DataWriter<WasmEcho>,
    reader: cyclonedds::DataReader<WasmEcho>,
}

fn make_peer(topic: &str) -> Peer {
    ensure_loopback_dds();
    let participant = cyclonedds::DomainParticipant::new(0).unwrap();
    let t = participant.create_topic::<WasmEcho>(topic).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let writer = publisher.create_writer(&t).unwrap();
    let reader = subscriber.create_reader(&t).unwrap();
    Peer {
        _participant: participant,
        _topic: t,
        writer,
        reader,
    }
}

/// Poll a native reader until a sample equal to `want` arrives.
fn wait_peer(reader: &cyclonedds::DataReader<WasmEcho>, want: &EchoMsg, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(samples) = reader.take() {
            for s in samples {
                if s.to_proto() == *want {
                    return true;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// Prove the server accepted and serves this connection before asserting
/// anything else: TCP `connect` returns at handshake (kernel backlog), not
/// at server accept. Without this, the first real assertion observes a
/// half-born connection on a slow accept loop instead of a served one.
fn hello_ack(client: &mut dds_wasm_bridge::BridgeClient) {
    use cyclonedds_proto::Control;
    client
        .send_control(&Control::Hello {
            proto: cyclonedds_proto::PROTO_VERSION,
            client: "phase-c-probe".into(),
        })
        .expect("probe hello send");
    let payload = client
        .recv_packet(Duration::from_secs(10))
        .expect("probe hello ack");
    let text = std::str::from_utf8(&payload).expect("probe ack utf-8");
    assert!(
        matches!(Control::from_json(text), Ok(Control::Ack { .. })),
        "probe hello must be acked, got {text:?}"
    );
}

fn bind_default(topic: &str) -> std::sync::Arc<WasmBridge> {
    ensure_loopback_dds();
    WasmBridge::bind(BridgeConfig {
        topic: topic.into(),
        ..Default::default()
    })
    .expect("gateway bind + DDS ready")
}

// ---------------------------------------------------------------------------
// 0. Wire compat: the pure-Rust browser codec is byte-identical to libddsc.
// ---------------------------------------------------------------------------

#[test]
fn codec_matches_native_libddsc_byte_for_byte() {
    use cyclonedds::{CdrDeserializer, CdrEncoding, CdrSerializer};
    let msg = sample();
    let pure = cyclonedds_proto::echo::encode_echo_xcdr1(&msg).unwrap();
    let native = CdrSerializer::<WasmEcho>::serialize(
        &WasmEcho::from_proto(&msg).unwrap(),
        CdrEncoding::Xcdr1,
    )
    .unwrap();
    assert_eq!(
        pure, native,
        "browser codec bytes must equal libddsc XCDR1 bytes"
    );
    // And back in the other direction.
    let via_native = CdrDeserializer::<WasmEcho>::deserialize(&pure, CdrEncoding::Xcdr1).unwrap();
    assert_eq!(via_native.to_proto(), msg);
    let via_pure = cyclonedds_proto::echo::decode_echo_xcdr1(&native).unwrap();
    assert_eq!(via_pure, msg);
}

// ---------------------------------------------------------------------------
// 1. browser -> CDR -> gateway -> native DDS peer
// ---------------------------------------------------------------------------

#[test]
fn browser_to_dds_cdr_binary() {
    let topic = unique_topic("phase_c_up");
    let bridge = bind_default(&topic);
    let peer = make_peer(&topic);
    // Let DDS discovery settle before the single shot.
    std::thread::sleep(Duration::from_millis(800));

    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    hello_ack(&mut client);
    let msg = sample();
    client.send_echo(&topic, &msg).unwrap();

    assert!(
        wait_peer(&peer.reader, &msg, Duration::from_secs(10)),
        "native peer never saw the browser CDR sample"
    );
    // The gateway also looped the sample back over DDS to this client.
    let mut errors = Vec::new();
    let (t, back, _) = client
        .recv_echo(Duration::from_secs(10), &mut errors)
        .expect("looped-back echo");
    assert_eq!(t, topic);
    assert_eq!(back, msg);
    assert!(errors.is_empty(), "unexpected error replies: {errors:?}");

    let stats = bridge.stats();
    assert!(stats.frames_in >= 1, "stats: {stats:?}");
    assert!(stats.samples_in >= 1, "stats: {stats:?}");
    assert_eq!(stats.errors_out, 0, "stats: {stats:?}");
}

// ---------------------------------------------------------------------------
// 2. native DDS peer -> gateway -> CDR -> browser
// ---------------------------------------------------------------------------

#[test]
fn dds_to_browser_cdr_binary() {
    let topic = unique_topic("phase_c_down");
    let bridge = bind_default(&topic);
    let peer = make_peer(&topic);
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    hello_ack(&mut client);
    std::thread::sleep(Duration::from_millis(800));

    let msg = EchoMsg {
        id: -3,
        text: String::new(),
        values: vec![i32::MIN, i32::MAX],
    };
    peer.writer
        .write(&WasmEcho::from_proto(&msg).unwrap())
        .unwrap();

    let mut errors = Vec::new();
    let (t, back, _seq) = client
        .recv_echo(Duration::from_secs(10), &mut errors)
        .expect("browser never got the DDS sample as CDR");
    assert_eq!(t, topic);
    assert_eq!(back, msg);
    assert!(errors.is_empty(), "unexpected error replies: {errors:?}");
    assert!(bridge.stats().samples_out >= 1, "{:?}", bridge.stats());
}

// ---------------------------------------------------------------------------
// 3. Legacy JSON rejected by default; connection survives; binary flows.
// ---------------------------------------------------------------------------

#[test]
fn legacy_json_rejected_by_default_then_binary_still_flows() {
    let topic = unique_topic("phase_c_nojson");
    let bridge = bind_default(&topic);
    let peer = make_peer(&topic);
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    hello_ack(&mut client);
    std::thread::sleep(Duration::from_millis(500));

    let msg = sample();
    client.send_legacy(&topic, &msg).unwrap();
    let payload = client
        .recv_packet(Duration::from_secs(5))
        .expect("expected an error reply, got nothing");
    let (code, _detail) =
        dds_wasm_bridge::parse_error_reply(&payload).expect("expected a typed error control reply");
    assert_eq!(code, "legacy_json_disabled");

    // Nothing reached DDS for the rejected envelope...
    assert!(
        !wait_peer(&peer.reader, &msg, Duration::from_millis(500)),
        "rejected JSON must not reach the DDS peer"
    );

    // ...but the connection is still alive: binary now flows.
    client.send_echo(&topic, &msg).unwrap();
    assert!(
        wait_peer(&peer.reader, &msg, Duration::from_secs(10)),
        "binary sample after rejection never arrived"
    );
    assert!(bridge.stats().errors_out >= 1, "{:?}", bridge.stats());
}

// ---------------------------------------------------------------------------
// 4. Explicit compat: JSON accepted, still forwarded as binary CDR.
// ---------------------------------------------------------------------------

#[test]
fn legacy_json_accepted_in_explicit_compat_and_forwarded_as_cdr() {
    ensure_loopback_dds();
    let topic = unique_topic("phase_c_json");
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: topic.clone(),
        legacy_json: true,
        ..Default::default()
    })
    .expect("compat gateway bind");
    let peer = make_peer(&topic);
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    hello_ack(&mut client);
    std::thread::sleep(Duration::from_millis(800));

    let msg = sample();
    client.send_legacy(&topic, &msg).unwrap();
    assert!(
        wait_peer(&peer.reader, &msg, Duration::from_secs(10)),
        "compat JSON never reached the DDS peer"
    );

    // The gateway looped that sample back through DDS: it must arrive as
    // a binary CDR frame even though it was published as JSON (data plane
    // normalizes to CDR in compat mode too).
    let mut errors = Vec::new();
    let (_, back, _) = client
        .recv_echo(Duration::from_secs(10), &mut errors)
        .expect("no looped-back CDR frame in compat mode");
    assert_eq!(back, msg);

    // Same for a natively written sample (DDS -> CDR -> client).
    let msg2 = EchoMsg {
        id: 99,
        text: "down".into(),
        values: vec![5],
    };
    peer.writer
        .write(&WasmEcho::from_proto(&msg2).unwrap())
        .unwrap();
    let (_, back2, _) = client
        .recv_echo(Duration::from_secs(10), &mut errors)
        .expect("no CDR frame in compat mode");
    assert_eq!(back2, msg2);
}

// ---------------------------------------------------------------------------
// 5. Observable error + timeout: truncated frame, unknown topic, idle read.
// ---------------------------------------------------------------------------

#[test]
fn truncated_frame_and_unknown_topic_are_typed_errors_then_idle_times_out() {
    use cyclonedds_proto::{DataFrame, FLAG_CDR_LE, PROTO_VERSION};
    let topic = unique_topic("phase_c_err");
    let bridge = bind_default(&topic);
    let _peer = make_peer(&topic);
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    hello_ack(&mut client);
    std::thread::sleep(Duration::from_millis(300));

    // Truncated DataFrame (length prefix says frame, bytes stop early).
    let mut full = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: topic.clone(),
        cdr: cyclonedds_proto::echo::encode_echo_xcdr1(&sample()).unwrap(),
        seq: 0,
    }
    .encode()
    .unwrap();
    full.truncate(6);
    client.send_raw(&full).unwrap();
    let payload = client.recv_packet(Duration::from_secs(5)).unwrap();
    let (code, _) = dds_wasm_bridge::parse_error_reply(&payload).unwrap();
    assert_eq!(code, "invalid_frame");

    // Well-formed frame for a topic this gateway does not serve.
    client.send_echo("no_such_topic", &sample()).unwrap();
    let payload = client.recv_packet(Duration::from_secs(5)).unwrap();
    let (code, _) = dds_wasm_bridge::parse_error_reply(&payload).unwrap();
    assert_eq!(code, "unknown_topic");

    assert!(bridge.stats().errors_out >= 2, "{:?}", bridge.stats());

    // Observable timeout: nothing else is coming on this connection.
    let start = Instant::now();
    let err = client
        .recv_packet(Duration::from_millis(300))
        .expect_err("idle connection must time out");
    assert!(
        matches!(err, dds_wasm_bridge::BridgeError::Timeout),
        "expected Timeout, got {err}"
    );
    assert!(start.elapsed() < Duration::from_secs(5));
}
