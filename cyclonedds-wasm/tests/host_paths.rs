//! Host-side paths of the browser-gateway client: every branch that runs
//! without a browser (wasm32 transport excluded by target gate).

use cyclonedds_proto::{ProtoError, Qos, Reliability};
use cyclonedds_wasm::{check_gateway_qos, WasmDdsError, WasmDomainParticipant};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Sample {
    id: i32,
    text: String,
}

fn participant() -> std::rc::Rc<WasmDomainParticipant> {
    WasmDomainParticipant::new("ws://127.0.0.1:9/dds").unwrap()
}

#[test]
fn display_names_every_error_variant() {
    assert_eq!(
        WasmDdsError::WebSocket("boom".into()).to_string(),
        "WebSocket error: boom"
    );
    assert_eq!(
        WasmDdsError::Serialization("bad".into()).to_string(),
        "serialization error: bad"
    );
    assert_eq!(WasmDdsError::NotConnected.to_string(), "not connected");
    assert_eq!(
        WasmDdsError::TopicNotFound("T".into()).to_string(),
        "topic not found: T"
    );
    assert_eq!(WasmDdsError::Timeout.to_string(), "timed out");
    assert_eq!(
        WasmDdsError::Unsupported("r".into()).to_string(),
        "unsupported: r"
    );
}

#[test]
fn from_proto_error_maps_every_arm() {
    let t = WasmDdsError::from(ProtoError::Unsupported("u".into())).to_string();
    assert!(t.contains("unsupported"), "{t}");
    let t = WasmDdsError::from(ProtoError::Serialization("s".into())).to_string();
    assert!(t.contains("serialization"), "{t}");
    let t = WasmDdsError::from(ProtoError::TooLarge { got: 10, max: 8 }).to_string();
    assert!(t.contains("frame too large: 10 (max 8)"), "{t}");
    let t = WasmDdsError::from(ProtoError::InvalidFrame("f")).to_string();
    assert!(t.contains("invalid frame: f"), "{t}");
    assert!(matches!(
        WasmDdsError::from(ProtoError::NotConnected),
        WasmDdsError::NotConnected
    ));
}

#[test]
fn participant_records_bridge_url_and_disconnects_quietly() {
    let p = participant();
    assert_eq!(p.url(), "ws://127.0.0.1:9/dds");
    p.disconnect();
    p.disconnect();
}

#[test]
fn host_reader_registers_and_enforces_qos() {
    let p = participant();
    let topic = p.create_topic::<Sample>("S").unwrap();
    let reader = p.create_reader(&topic, Box::new(|_: Sample| {})).unwrap();
    assert_eq!(reader.topic_name(), "S");
    let reader2 = p
        .create_reader_with_qos(&topic, &Qos::default(), Box::new(|_: Sample| {}))
        .unwrap();
    assert_eq!(reader2.topic_name(), "S");
    let reliable = Qos {
        reliability: Reliability::Reliable,
        ..Qos::default()
    };
    assert!(matches!(
        p.create_reader_with_qos(&topic, &reliable, Box::new(|_: Sample| {})),
        Err(WasmDdsError::Unsupported(_))
    ));
}

#[test]
fn host_writer_validates_envelope_then_reports_not_connected() {
    let p = participant();
    let topic = p.create_topic::<Sample>("S").unwrap();
    let writer = p.create_writer(&topic).unwrap();
    let sample = Sample {
        id: 1,
        text: "hi".into(),
    };
    assert!(matches!(
        writer.write(&sample),
        Err(WasmDdsError::NotConnected)
    ));
    let reliable = Qos {
        reliability: Reliability::Reliable,
        ..Qos::default()
    };
    assert!(matches!(
        p.create_writer_with_qos(&topic, &reliable),
        Err(WasmDdsError::Unsupported(_))
    ));
}

#[test]
fn echo_writer_validates_framing_reports_gap_and_names_topic() {
    use cyclonedds_proto::echo::EchoMsg;
    let p = participant();
    let topic = p.create_topic::<EchoMsg>("WasmEcho").unwrap();
    let writer = p.create_echo_writer(&topic).unwrap();
    assert_eq!(writer.topic_name(), "WasmEcho");
    let msg = EchoMsg {
        id: 7,
        text: "e".into(),
        values: vec![1],
    };
    assert!(matches!(
        writer.write_echo(&msg),
        Err(WasmDdsError::NotConnected)
    ));
    assert!(matches!(
        writer.write_echo(&msg),
        Err(WasmDdsError::NotConnected)
    ));
    let reader = p
        .create_echo_reader(&topic, Box::new(|_: EchoMsg| {}))
        .unwrap();
    assert_eq!(reader.topic_name(), "WasmEcho");
}

#[test]
fn gateway_qos_accepts_default_rejects_reliable() {
    check_gateway_qos(&Qos::default()).unwrap();
    let reliable = Qos {
        reliability: Reliability::Reliable,
        ..Qos::default()
    };
    assert!(matches!(
        check_gateway_qos(&reliable),
        Err(WasmDdsError::Unsupported(_))
    ));
}
