//! Shared harness for the gateway E2E scenarios: live [`WasmBridge`]
//! instances on `127.0.0.1:0`, one per test, with unique DDS topics so
//! tests stay parallel-safe on the shared domain.
//!
//! DDS transport is pinned to loopback (same sandbox-proofing as the
//! bridge's own suites): the stock machine selector may name a NIC that
//! does not exist here, failing every `DomainParticipant::new`.

use cyclonedds_proto::echo::EchoMsg;
use dds_wasm_bridge::{BridgeClient, BridgeConfig, WasmBridge, WasmEcho};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Loopback interface name is OS-specific (`lo0` on macOS).
#[cfg(target_os = "macos")]
const LO_IF: &str = "lo0";
#[cfg(not(target_os = "macos"))]
const LO_IF: &str = "lo";

/// Loopback-pinned CYCLONEDDS_URI for this OS.
fn lo_uri() -> String {
    format!(
        r#"<CycloneDDS><Domain><General><Interfaces><NetworkInterface name="{LO_IF}"/></Interfaces></General></Domain></CycloneDDS>"#
    )
}

/// Pin loopback DDS once per process (mirrors the bridge suites).
pub fn ensure_loopback_dds() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::env::set_var("CYCLONEDDS_URI", crate::lo_uri());
    });
}

/// Unique DDS topic per test: parallel-safe on the shared domain.
pub fn unique_topic(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}_{}_{}", std::process::id(), nanos)
}

/// Bind a gateway serving exactly `topic` (JSON data plane off).
pub fn bind_gateway(topic: &str) -> Arc<WasmBridge> {
    ensure_loopback_dds();
    let config = BridgeConfig {
        topic: topic.to_string(),
        ..BridgeConfig::default()
    };
    WasmBridge::bind(config).expect("gateway binds on loopback")
}

/// Bind a gateway with legacy JSON compat on.
pub fn bind_gateway_legacy(topic: &str) -> Arc<WasmBridge> {
    ensure_loopback_dds();
    let config = BridgeConfig {
        topic: topic.to_string(),
        legacy_json: true,
        ..BridgeConfig::default()
    };
    WasmBridge::bind(config).expect("legacy gateway binds on loopback")
}

/// Native DDS peer on `topic` (samples in both directions).
pub struct Peer {
    _participant: cyclonedds::DomainParticipant,
    _topic: cyclonedds::Topic<WasmEcho>,
    pub writer: cyclonedds::DataWriter<WasmEcho>,
    pub reader: cyclonedds::DataReader<WasmEcho>,
}

/// Build a native peer (mirrors the bridge suites).
pub fn make_peer(topic: &str) -> Peer {
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

/// Proto sample used across scenarios.
pub fn sample(id: i32) -> EchoMsg {
    EchoMsg {
        id,
        text: format!("e2e-{id}"),
        values: vec![id, -id],
    }
}

/// Raw length-prefixed packet write (control / adversarial bytes).
pub fn send_packet(sock: &mut TcpStream, payload: &[u8]) {
    sock.write_all(&(payload.len() as u32).to_le_bytes())
        .unwrap();
    sock.write_all(payload).unwrap();
    sock.flush().unwrap();
}

/// Raw length-prefixed packet read with a bounded wait.
pub fn recv_packet(sock: &mut TcpStream, timeout: Duration) -> Option<Vec<u8>> {
    sock.set_read_timeout(Some(timeout)).ok()?;
    let mut len_buf = [0u8; 4];
    sock.read_exact(&mut len_buf).ok()?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > 64 * 1024 * 1024 {
        return None;
    }
    let mut payload = vec![0u8; len];
    sock.read_exact(&mut payload).ok()?;
    Some(payload)
}

/// Client connected to a live gateway.
pub fn connect(bridge: &WasmBridge) -> BridgeClient {
    BridgeClient::connect(bridge.addr()).expect("client connects")
}
