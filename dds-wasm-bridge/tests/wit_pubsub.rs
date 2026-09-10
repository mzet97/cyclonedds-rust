//! Phase F: WASI guest (`tese:dds-bridge@0.1.0`) against a native peer
//! through the reference host (this gateway).
//!
//! The guest here is `dds-wasm-guest`: the portable WIT `guest`-world
//! projection (WIT `echo-sample` <-> [`WitEchoSample`]). The host boundary
//! is the WIT surface — frames in, frames out, no handles cross it.
//!
//! Sandbox note (recorded gap, not discretized): `wasmtime` is not
//! installed in this environment, so the guest runs in-process instead of
//! inside a component sandbox. Isolation therefore rests on the type
//! boundary (guest graph contains no `cyclonedds-rust-sys` — asserted in
//! `dds-wasm-guest` tests) plus host-side re-validation of every guest
//! byte, not on hardware/OS isolation yet.

use dds_wasm_bridge::{BridgeClient, BridgeConfig, WasmBridge, WasmEcho};
use dds_wasm_guest::{GuestDispatcher, WitEchoSample};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Loopback interface name is OS-specific (`lo0` on macOS, where plain
/// `lo` matches nothing and every participant creation fails).
#[cfg(target_os = "macos")]
const LO_IF: &str = "lo0";
#[cfg(not(target_os = "macos"))]
const LO_IF: &str = "lo";

fn ensure_loopback_dds() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var(
            "CYCLONEDDS_URI",
            format!(
                r#"<CycloneDDS><Domain><General><Interfaces><NetworkInterface name="{LO_IF}"/></Interfaces></General></Domain></CycloneDDS>"#
            ),
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

/// Native reference host: owns DDS entities, moves guest bytes both ways.
struct NativeHost {
    _participant: cyclonedds::DomainParticipant,
    _topic: cyclonedds::Topic<WasmEcho>,
    writer: cyclonedds::DataWriter<WasmEcho>,
    reader: cyclonedds::DataReader<WasmEcho>,
}

fn make_host(topic: &str) -> NativeHost {
    ensure_loopback_dds();
    let participant = cyclonedds::DomainParticipant::new(0).unwrap();
    let t = participant.create_topic::<WasmEcho>(topic).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let writer = publisher.create_writer(&t).unwrap();
    let reader = subscriber.create_reader(&t).unwrap();
    NativeHost {
        _participant: participant,
        _topic: t,
        writer,
        reader,
    }
}

#[test]
fn guest_publishes_to_native_peer_through_host() {
    let topic = unique_topic("phase_f_pub");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: topic.clone(),
        ..Default::default()
    })
    .unwrap();
    let host = make_host(&topic);
    std::thread::sleep(Duration::from_millis(800));

    // Guest side: WIT record -> staged wire frame (WIT `encode-frame`).
    let mut guest = GuestDispatcher::new(&[topic.as_str()], 8);
    let sample = WitEchoSample {
        id: 7,
        text: "via-guest".into(),
        values: vec![1, 2],
    };
    guest.encode_frame(&topic, &sample).unwrap();
    assert_eq!(guest.outbox_stats(), (1, 0));

    // Host transport: staged bytes cross the socket to the gateway.
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    for frame in guest.drain_outbox() {
        client.send_raw(&frame).unwrap();
    }

    // Native peer observes the guest sample (WIT record == DDS sample).
    let start = Instant::now();
    let seen = loop {
        let mut found = None;
        if let Ok(samples) = host.reader.take() {
            found = samples.into_iter().find(|s| s.id == 7);
        }
        if found.is_some() || start.elapsed() > Duration::from_secs(10) {
            break found;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let seen = seen.expect("native peer never saw the guest sample");
    assert_eq!(seen.text, "via-guest");
    assert_eq!(seen.values.to_vec(), vec![1, 2]);
}

#[test]
fn native_peer_publishes_to_guest_subscription() {
    let topic = unique_topic("phase_f_sub");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: topic.clone(),
        ..Default::default()
    })
    .unwrap();
    let host = make_host(&topic);
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    std::thread::sleep(Duration::from_millis(800));

    // Native -> gateway -> socket -> guest `handle-frame`.
    host.writer
        .write(&WasmEcho::from_proto(&dds_wasm_guest_proto_sample(9)).unwrap())
        .unwrap();
    let mut guest = GuestDispatcher::new(&[topic.as_str()], 8);
    let start = Instant::now();
    let decoded = loop {
        let packet = client.recv_packet(Duration::from_secs(10)).unwrap();
        // Control/error replies are JSON; data frames are binary.
        if packet.first() == Some(&b'{') {
            continue;
        }
        match guest.handle_frame(&packet).unwrap() {
            Some(got) => break got,
            None => continue,
        }
    };
    assert!(start.elapsed() < Duration::from_secs(10));
    assert_eq!(decoded.0, topic);
    assert_eq!(decoded.1.id, 9);
    assert_eq!(decoded.1.text, "from-native");
}

fn dds_wasm_guest_proto_sample(id: i32) -> cyclonedds_proto::echo::EchoMsg {
    cyclonedds_proto::echo::EchoMsg {
        id,
        text: "from-native".into(),
        values: vec![9],
    }
}

#[test]
fn host_revalidates_every_guest_byte() {
    // A malicious/buggy guest speaking out-of-contract bytes gets typed
    // errors, never a gateway crash, never silent delivery.
    let topic = unique_topic("phase_f_adv");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: topic.clone(),
        ..Default::default()
    })
    .unwrap();
    let host = make_host(&topic);
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    std::thread::sleep(Duration::from_millis(300));

    // Garbage bytes: typed error, connection survives.
    client.send_raw(&[0xFF, 0x00, 0x01]).unwrap();
    let payload = client.recv_packet(Duration::from_secs(5)).unwrap();
    let (code, _) = dds_wasm_bridge::parse_error_reply(&payload).expect("typed error");
    assert_eq!(code, "invalid_frame");

    // Well-formed frame, wrong topic: rejected before touching DDS.
    let mut guest = GuestDispatcher::new(&["elsewhere"], 8);
    guest
        .encode_frame(
            "elsewhere",
            &WitEchoSample {
                id: 1,
                text: "x".into(),
                values: vec![],
            },
        )
        .unwrap();
    for frame in guest.drain_outbox() {
        client.send_raw(&frame).unwrap();
    }
    let payload = client.recv_packet(Duration::from_secs(5)).unwrap();
    let (code, _) = dds_wasm_bridge::parse_error_reply(&payload).expect("typed error");
    assert_eq!(code, "unknown_topic");

    // Nothing reached the native peer.
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        host.reader.take().unwrap().is_empty(),
        "adversarial bytes must not reach DDS"
    );
}
