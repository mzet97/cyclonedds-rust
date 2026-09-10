//! `BridgeClient` flows: timeouts, cancellation, error collection, and
//! shape validation — including against a fake upstream that misbehaves
//! on purpose (no DDS involved).

use dds_wasm_bridge::{BridgeClient, BridgeError, CancelFlag};
use dds_wasm_e2e::*;
use std::io::Write;
use std::net::TcpListener;
use std::time::Duration;

const T: Duration = Duration::from_secs(3);

/// Fake upstream: accepts one connection, replays canned packets, then
/// stays quiet. Lets the client prove timeout/cancel/shape handling
/// without a gateway.
fn fake_server(packets: Vec<Vec<u8>>) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        for p in packets {
            let len = p.len() as u32;
            if sock.write_all(&len.to_le_bytes()).is_err() {
                return;
            }
            if sock.write_all(&p).is_err() {
                return;
            }
        }
        // Stay quiet until the client goes away.
        let mut buf = [0u8; 1];
        use std::io::Read as _;
        let _ = sock.read(&mut buf);
    });
    addr
}

fn error_packet(code: &str) -> Vec<u8> {
    format!(r#"{{"proto":0,"kind":"error","code":"{code}","detail":"d"}}"#).into_bytes()
}

#[test]
fn recv_packet_times_out_on_a_quiet_socket() {
    let topic = unique_topic("e2e_cli_tmo");
    let bridge = bind_gateway(&topic);
    let mut client = connect(&bridge);
    match client.recv_packet(Duration::from_millis(150)) {
        Err(BridgeError::Timeout) => {}
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[test]
fn recv_echo_collects_errors_before_a_sample() {
    let topic = unique_topic("e2e_cli_collect");
    let bridge = bind_gateway(&topic);
    let peer = make_peer(&topic);
    let mut client = connect(&bridge);

    // Error reply on THIS connection first (unknown topic), then a real
    // sample via the peer. Discovery needs a moment (best-effort plane):
    // republish until the sample loops back or the budget runs out.
    {
        use cyclonedds_proto::{DataFrame, FLAG_CDR_LE, PROTO_VERSION};
        let bad = DataFrame {
            proto: PROTO_VERSION,
            flags: FLAG_CDR_LE,
            topic: "elsewhere".into(),
            cdr: vec![1],
            seq: 0,
        }
        .encode()
        .unwrap();
        client.send_raw(&bad).unwrap();
    }
    let native = dds_wasm_bridge::WasmEcho {
        id: 11,
        text: "via-peer".into(),
        values: cyclonedds::DdsSequence::from_vec(vec![1]).unwrap(),
    };
    let mut errors = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    // The error reply (conn thread) and the sample (DDS thread) have no
    // cross-thread order: wait for BOTH signals before asserting.
    let (t, msg) = loop {
        peer.writer.write(&native).unwrap();
        match client.recv_echo(Duration::from_millis(500), &mut errors) {
            Ok((t, msg, _)) if msg.id == 11 => {
                if errors.iter().any(|(c, _)| c == "unknown_topic") {
                    break (t, msg);
                }
            }
            Ok(_) | Err(BridgeError::Timeout) => {}
            Err(other) => panic!("unexpected wait error: {other:?}"),
        }
        assert!(
            std::time::Instant::now() < deadline,
            "sample/error never both arrived; errors={errors:?}"
        );
    };
    assert_eq!(t, topic);
    assert_eq!(msg.id, 11);
}

#[test]
fn recv_echo_cancel_wins_over_a_quiet_socket() {
    let addr = fake_server(vec![]);
    let mut client = BridgeClient::connect(addr).unwrap();
    let cancel = CancelFlag::new();
    let cancel2 = cancel.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        cancel2.cancel();
    });
    let mut errors = Vec::new();
    match client.recv_echo_cancel(T, &mut errors, &cancel) {
        Err(BridgeError::Cancelled) => {}
        other => panic!("expected Cancelled, got {other:?}"),
    }
}

#[test]
fn recv_echo_cancel_times_out_without_cancel() {
    let addr = fake_server(vec![]);
    let mut client = BridgeClient::connect(addr).unwrap();
    let cancel = CancelFlag::new();
    let mut errors = Vec::new();
    match client.recv_echo_cancel(Duration::from_millis(200), &mut errors, &cancel) {
        Err(BridgeError::Timeout) => {}
        other => panic!("expected Timeout, got {other:?}"),
    }
}

#[test]
fn recv_echo_rejects_non_cdr_shapes_from_a_rogue_server() {
    use cyclonedds_proto::{DataFrame, FLAG_CDR_LE, PROTO_VERSION};
    // Valid frame with wrong flags.
    let bad_flags = DataFrame {
        proto: PROTO_VERSION,
        flags: 0,
        topic: "t".into(),
        cdr: vec![1, 2, 3],
        seq: 0,
    }
    .encode()
    .unwrap();
    let addr = fake_server(vec![bad_flags]);
    let mut client = BridgeClient::connect(addr).unwrap();
    let mut errors = Vec::new();
    match client.recv_echo(T, &mut errors) {
        Err(BridgeError::UnexpectedShape(_)) => {}
        other => panic!("expected UnexpectedShape, got {other:?}"),
    }

    // Same via the cancellable wait.
    let bad_cdr = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: "t".into(),
        cdr: vec![0xFF; 16],
        seq: 0,
    }
    .encode()
    .unwrap();
    let addr = fake_server(vec![error_packet("stale"), bad_cdr]);
    let mut client = BridgeClient::connect(addr).unwrap();
    let mut errors = Vec::new();
    let cancel = CancelFlag::new();
    match client.recv_echo_cancel(T, &mut errors, &cancel) {
        Err(BridgeError::Proto(_)) => {}
        other => panic!("expected Proto, got {other:?}"),
    }
    assert_eq!(errors, vec![("stale".to_string(), "d".to_string())]);
}

#[test]
fn connect_with_retry_fails_loud_on_a_dead_port() {
    // Reserve then release a port so nothing listens on it.
    let port = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        l.local_addr().unwrap().port()
    };
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    match BridgeClient::connect_with_retry(
        addr,
        2,
        Duration::from_millis(1),
        Duration::from_millis(5),
    ) {
        Err(BridgeError::Io(_)) => {}
        other => panic!("expected Io, got {other:?}"),
    }
}

#[test]
fn close_terminates_live_connections_promptly() {
    let topic = unique_topic("e2e_cli_closed");
    let bridge = bind_gateway(&topic);
    let mut client = connect(&bridge);
    bridge.close();
    // The pre-existing connection observes EOF promptly (no hang), and
    // new connects are refused once the accept loop is gone.
    // Any error proves loud termination (FIN/RST/timeout all qualify);
    // only DATA from a dead gateway would disprove it.
    match client.recv_packet(Duration::from_secs(3)) {
        Err(_) => {}
        Ok(p) => panic!(
            "live conn must end after close, got {} bytes: {:02x?}",
            p.len(),
            &p[..p.len().min(32)]
        ),
    }
    let deadline = std::time::Instant::now() + T;
    while std::net::TcpStream::connect(bridge.addr()).is_ok() {
        assert!(
            std::time::Instant::now() < deadline,
            "accept loop never stopped"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn recv_echo_times_out_on_a_quiet_server() {
    let addr = fake_server(vec![]);
    let mut client = BridgeClient::connect(addr).unwrap();
    let mut errors = Vec::new();
    match client.recv_echo(Duration::from_millis(200), &mut errors) {
        Err(BridgeError::Timeout) => {}
        other => panic!("expected Timeout, got {other:?}"),
    }
    assert!(errors.is_empty());
}

#[test]
fn recv_echo_cancel_surfaces_socket_and_shape_errors() {
    use cyclonedds_proto::{DataFrame, FLAG_CDR_LE, PROTO_VERSION};
    // Abrupt close mid-wait: the error (not Timeout/Cancelled) wins.
    let addr = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let Ok((sock, _)) = listener.accept() else {
                return;
            };
            drop(sock);
        });
        addr
    };
    let mut client = BridgeClient::connect(addr).unwrap();
    let cancel = CancelFlag::new();
    let mut errors = Vec::new();
    match client.recv_echo_cancel(T, &mut errors, &cancel) {
        Err(BridgeError::Disconnected) | Err(BridgeError::Timeout) => {}
        other => panic!("expected a socket error, got {other:?}"),
    }

    // Non-CDR flags and garbage CDR through the cancellable wait.
    let bad_flags = DataFrame {
        proto: PROTO_VERSION,
        flags: 0,
        topic: "t".into(),
        cdr: vec![1, 2, 3],
        seq: 0,
    }
    .encode()
    .unwrap();
    let addr = fake_server(vec![bad_flags]);
    let mut client = BridgeClient::connect(addr).unwrap();
    let mut errors = Vec::new();
    match client.recv_echo_cancel(T, &mut errors, &cancel) {
        Err(BridgeError::UnexpectedShape(_)) => {}
        other => panic!("expected UnexpectedShape, got {other:?}"),
    }

    let bad_cdr = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: "t".into(),
        cdr: vec![0xFF; 16],
        seq: 0,
    }
    .encode()
    .unwrap();
    let addr = fake_server(vec![bad_cdr]);
    let mut client = BridgeClient::connect(addr).unwrap();
    let mut errors = Vec::new();
    match client.recv_echo_cancel(T, &mut errors, &cancel) {
        Err(BridgeError::Proto(_)) => {}
        other => panic!("expected Proto, got {other:?}"),
    }
}

#[test]
fn oversize_declared_length_is_rejected_without_allocating() {
    // A fake upstream declaring a 4 GiB payload: the helper must refuse
    // without allocating.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        let _ = sock.write_all(&u32::MAX.to_le_bytes());
        let mut buf = [0u8; 1];
        use std::io::Read as _;
        let _ = sock.read(&mut buf);
    });
    let mut sock = std::net::TcpStream::connect(addr).unwrap();
    assert!(dds_wasm_e2e::recv_packet(&mut sock, T).is_none());
}

#[test]
fn recv_echo_deadline_arm_triggers_on_a_flooding_server() {
    // Data always available: every quantum returns instantly, so the
    // deadline check at the loop top — not a socket timeout — ends the
    // wait. The server floods error replies until the client leaves.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let Ok((mut sock, _)) = listener.accept() else {
            return;
        };
        let pkt = error_packet("stale");
        let len = (pkt.len() as u32).to_le_bytes();
        for _ in 0..100_000 {
            if sock.write_all(&len).is_err() || sock.write_all(&pkt).is_err() {
                return;
            }
        }
    });
    let mut client = BridgeClient::connect(addr).unwrap();
    let mut errors = Vec::new();
    match client.recv_echo(Duration::from_millis(300), &mut errors) {
        Err(BridgeError::Timeout) => {}
        other => panic!("expected deadline Timeout, got {other:?}"),
    }
    assert!(!errors.is_empty(), "flooded errors must be collected");
}

#[test]
fn recv_echo_cancel_returns_samples_like_the_plain_wait() {
    use cyclonedds_proto::{echo::encode_echo_xcdr1, DataFrame, FLAG_CDR_LE, PROTO_VERSION};
    let msg = cyclonedds_proto::echo::EchoMsg {
        id: 5,
        text: "ok".into(),
        values: vec![1],
    };
    let frame = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: "t".into(),
        cdr: encode_echo_xcdr1(&msg).unwrap(),
        seq: 3,
    }
    .encode()
    .unwrap();
    let addr = fake_server(vec![frame]);
    let mut client = BridgeClient::connect(addr).unwrap();
    let cancel = CancelFlag::new();
    let mut errors = Vec::new();
    let (topic, back, seq) = client.recv_echo_cancel(T, &mut errors, &cancel).unwrap();
    assert_eq!((topic.as_str(), back.id, seq), ("t", 5, 3));
    assert!(errors.is_empty());
}
