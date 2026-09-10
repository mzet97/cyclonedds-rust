//! Gateway lifecycle: handshake, subscription routing, compat modes,
//! dead-owner behavior, and clean shutdown.

use cyclonedds_proto::{Control, Qos, Reliability, PROTO_VERSION};
use dds_wasm_e2e::*;
use std::time::Duration;

const T: Duration = Duration::from_secs(5);

#[test]
fn hello_and_register_are_acknowledged_reliable_is_refused() {
    let topic = unique_topic("e2e_life_hs");
    let bridge = bind_gateway(&topic);
    let mut client = connect(&bridge);

    let hello = Control::Hello {
        proto: PROTO_VERSION,
        client: "e2e".into(),
    };
    client.send_control(&hello).unwrap();
    // An ack is not a sample: read the raw packet instead.
    let mut raw = std::net::TcpStream::connect(bridge.addr()).unwrap();
    send_packet(&mut raw, hello.to_json().unwrap().as_bytes());
    let reply = recv_packet(&mut raw, T).expect("ack");
    assert!(dds_wasm_bridge::parse_error_reply(&reply).is_none());
    match Control::from_json(std::str::from_utf8(&reply).unwrap()).unwrap() {
        Control::Ack { seq, .. } => assert_eq!(seq, 0),
        other => panic!("expected ack, got {:?}", std::mem::discriminant(&other)),
    }

    let reg = Control::Register {
        proto: PROTO_VERSION,
        topic: topic.clone(),
        type_name: "WasmEcho".into(),
        qos: Qos::default(),
        seq: 7,
    };
    client.send_control(&reg).unwrap();

    let bad = Control::Register {
        proto: PROTO_VERSION,
        topic: topic.clone(),
        type_name: "WasmEcho".into(),
        qos: Qos {
            reliability: Reliability::Reliable,
            ..Qos::default()
        },
        seq: 8,
    };
    let mut raw = std::net::TcpStream::connect(bridge.addr()).unwrap();
    send_packet(&mut raw, bad.to_json().unwrap().as_bytes());
    let reply = recv_packet(&mut raw, T).expect("error reply");
    let (code, _) = dds_wasm_bridge::parse_error_reply(&reply).expect("typed error");
    assert_eq!(code, "unsupported_qos");
}

#[test]
fn unsubscribe_stops_delivery_while_client_stays_connected() {
    let topic = unique_topic("e2e_life_unsub");
    let bridge = bind_gateway(&topic);
    let peer = make_peer(&topic);
    let mut client = connect(&bridge);

    // Default subscription receives the peer sample. Discovery needs a
    // moment (best-effort plane): republish until it loops back.
    let native = dds_wasm_bridge::WasmEcho {
        id: 21,
        text: "before".into(),
        values: cyclonedds::DdsSequence::from_vec(vec![1]).unwrap(),
    };
    let mut errors = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        peer.writer.write(&native).unwrap();
        match client.recv_echo(Duration::from_millis(500), &mut errors) {
            Ok((_, msg, _)) if msg.id == 21 => break,
            Ok(_) | Err(dds_wasm_bridge::BridgeError::Timeout) => {}
            Err(other) => panic!("unexpected wait error: {other:?}"),
        }
        assert!(
            std::time::Instant::now() < deadline,
            "pre-unsub sample never arrived"
        );
    }

    // Unsubscribe: further peer samples never arrive here.
    let unsub = Control::Unsubscribe {
        proto: PROTO_VERSION,
        topic: topic.clone(),
        seq: 1,
    };
    client.send_control(&unsub).unwrap();
    // Wait out any in-flight frame, then publish again.
    std::thread::sleep(Duration::from_millis(300));
    // Drain anything already queued.
    let mut errors = Vec::new();
    while client
        .recv_echo(Duration::from_millis(200), &mut errors)
        .is_ok()
    {}
    peer.writer
        .write(&dds_wasm_bridge::WasmEcho {
            id: 22,
            text: "after".into(),
            values: cyclonedds::DdsSequence::from_vec(vec![2]).unwrap(),
        })
        .unwrap();
    let mut errors = Vec::new();
    match client.recv_echo(Duration::from_millis(600), &mut errors) {
        Err(dds_wasm_bridge::BridgeError::Timeout) => {}
        other => panic!(
            "unsubscribed client must time out, got {:?}",
            std::mem::discriminant(&other)
        ),
    }
}

#[test]
fn legacy_compat_off_rejects_on_stays_usable() {
    let topic = unique_topic("e2e_life_legacy");
    let bridge = bind_gateway(&topic);
    let mut client = connect(&bridge);

    client.send_legacy(&topic, &sample(1)).unwrap();
    let mut errors = Vec::new();
    // The rejection is an error reply; the binary plane still works after.
    // No cross-thread order between the two: wait for BOTH signals.
    client.send_echo(&topic, &sample(2)).unwrap();
    let peer = make_peer(&topic);
    let deadline = std::time::Instant::now() + T;
    let (t, msg) = loop {
        match client.recv_echo(Duration::from_millis(500), &mut errors) {
            Ok((t, msg, _)) if msg.id == 2 => {
                if errors.iter().any(|(c, _)| c == "legacy_json_disabled") {
                    break (t, msg);
                }
            }
            Ok(_) | Err(dds_wasm_bridge::BridgeError::Timeout) => {}
            Err(other) => panic!("unexpected wait error: {other:?}"),
        }
        assert!(
            std::time::Instant::now() < deadline,
            "binary sample never looped back"
        );
    };
    assert_eq!(t, topic);
    // The legacy sample never reached DDS: only the binary one loops back.
    assert_eq!(msg.id, 2);
    let _ = peer;
}

#[test]
fn legacy_compat_on_forwards_json_as_cdr() {
    let topic = unique_topic("e2e_life_legacy_on");
    let bridge = bind_gateway_legacy(&topic);
    let mut client = connect(&bridge);
    let peer = make_peer(&topic);

    client.send_legacy(&topic, &sample(31)).unwrap();
    let mut errors = Vec::new();
    let (t, msg, _) = client.recv_echo(T, &mut errors).unwrap();
    assert_eq!((t.as_str(), msg.id), (topic.as_str(), 31));
    assert!(errors.is_empty(), "{errors:?}");
    let _ = peer;
}

#[test]
fn closed_gateway_releases_its_port() {
    let topic = unique_topic("e2e_life_dead");
    let bridge = bind_gateway(&topic);
    bridge.close();
    // Rebinding the exact port proves `close()` dropped the listener.
    // The accept thread holds a cloned handle, so poll until it unwinds
    // (bounded: the loop observes shutdown every 5 ms). Refused connects
    // + EOF on live conns are covered by
    // `close_terminates_live_connections_promptly`.
    let deadline = std::time::Instant::now() + T;
    loop {
        match std::net::TcpListener::bind(bridge.addr()) {
            Ok(_guard) => break,
            Err(_) => {
                assert!(std::time::Instant::now() < deadline, "port never released");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

#[test]
fn dropping_the_bridge_releases_its_port() {
    let topic = unique_topic("e2e_life_drop");
    let addr = {
        let bridge = bind_gateway(&topic);
        bridge.addr()
    };
    // Rebinding the exact port proves the listener died with the bridge
    // (`Drop` runs the full `close()`). Poll: the accept thread holds a
    // cloned handle until it observes shutdown.
    let deadline = std::time::Instant::now() + T;
    loop {
        match std::net::TcpListener::bind(addr) {
            Ok(_guard) => break,
            Err(_) => {
                assert!(std::time::Instant::now() < deadline, "port never released");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

#[test]
fn setup_failure_is_reported_not_hung() {
    use dds_wasm_bridge::BridgeConfig;
    ensure_loopback_dds();
    // Empty topic names are rejected by DDS topic creation: the gateway
    // must report the error instead of hanging on readiness.
    let config = BridgeConfig {
        topic: "".to_string(),
        ..BridgeConfig::default()
    };
    match dds_wasm_bridge::WasmBridge::bind(config) {
        Err(_) => {}
        Ok(_) => panic!("empty topic must fail setup"),
    }
}

#[test]
fn oversize_native_sample_is_skipped_without_stalling_the_lane() {
    let topic = unique_topic("e2e_life_bigskip");
    let bridge = bind_gateway(&topic);
    let peer = make_peer(&topic);
    let mut client = connect(&bridge);

    // Native writers are not bound by the portable 1 MiB cap: the lane
    // must skip the un-encodable sample and keep flowing.
    let big = dds_wasm_bridge::WasmEcho {
        id: 41,
        text: "x".repeat(2 * 1024 * 1024),
        values: cyclonedds::DdsSequence::from_vec(vec![]).unwrap(),
    };
    peer.writer.write(&big).unwrap();
    let small = dds_wasm_bridge::WasmEcho {
        id: 42,
        text: "small".into(),
        values: cyclonedds::DdsSequence::from_vec(vec![7]).unwrap(),
    };
    let deadline = std::time::Instant::now() + T;
    let mut errors = Vec::new();
    loop {
        peer.writer.write(&small).unwrap();
        match client.recv_echo(Duration::from_millis(500), &mut errors) {
            Ok((_, msg, _)) if msg.id == 42 => break,
            Ok(_) | Err(dds_wasm_bridge::BridgeError::Timeout) => {}
            Err(other) => panic!("unexpected wait error: {other:?}"),
        }
        assert!(
            std::time::Instant::now() < deadline,
            "lane stalled on big sample"
        );
    }
}

#[test]
fn shutdown_under_load_is_clean_and_releases_the_port() {
    use dds_wasm_bridge::BridgeClient;
    let topic = unique_topic("e2e_life_soak");
    let bridge = bind_gateway(&topic);
    // Saturate: several clients publishing as fast as TCP allows.
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let workers: Vec<_> = (0..4)
        .map(|i| {
            let addr = bridge.addr();
            let t = topic.clone();
            let stop = stop.clone();
            std::thread::spawn(move || {
                let mut client = BridgeClient::connect(addr).unwrap();
                let mut n = 0u32;
                while !stop.load(std::sync::atomic::Ordering::Relaxed) && n < 500 {
                    if client.send_echo(&t, &sample(i * 1000 + n as i32)).is_err() {
                        break;
                    }
                    n += 1;
                }
            })
        })
        .collect();
    std::thread::sleep(Duration::from_millis(300));
    bridge.close();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    for w in workers {
        w.join().expect("worker joins");
    }
    // Port released (accept thread unwound) within budget; live conns ended.
    let deadline = std::time::Instant::now() + T;
    loop {
        match std::net::TcpListener::bind(bridge.addr()) {
            Ok(_guard) => break,
            Err(_) => {
                assert!(std::time::Instant::now() < deadline, "port never released");
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}
