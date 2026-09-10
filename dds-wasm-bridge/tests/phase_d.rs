//! Phase D: per-connection dispatcher, multi-topic lanes, bounded queues
//! with explicit overflow, reconnect backoff, cancellation, clean drop.
//!
//! Same harness as Phase C: [`BridgeClient`] replays the exact `DataFrame`
//! bytes the wasm32 client emits; only the socket type differs.

use cyclonedds_proto::echo::EchoMsg;
use cyclonedds_proto::{Control, Durability, Qos, Reliability, PROTO_VERSION};
use dds_wasm_bridge::{
    backoff_delay, BridgeClient, BridgeConfig, BridgeError, CancelFlag, WasmBridge, WasmEcho,
};
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

fn sample(id: i32) -> EchoMsg {
    EchoMsg {
        id,
        text: format!("d-{id}"),
        values: vec![id, -id],
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

/// Read one packet and expect `ack` with this `seq`.
fn expect_ack(client: &mut BridgeClient, seq: u32) {
    let payload = client
        .recv_packet(Duration::from_secs(5))
        .expect("ack packet");
    let text = std::str::from_utf8(&payload).expect("ack is JSON");
    match Control::from_json(text).expect("control parses") {
        Control::Ack { seq: got, .. } => assert_eq!(got, seq, "ack seq"),
        other => panic!("expected ack, got {:?}", std::mem::discriminant(&other)),
    }
}

/// Read one packet and expect a typed `error` with this `code`.
fn expect_error(client: &mut BridgeClient, code: &str) {
    let payload = client
        .recv_packet(Duration::from_secs(5))
        .expect("error packet");
    let (got, _) = dds_wasm_bridge::parse_error_reply(&payload).expect("typed error");
    assert_eq!(got, code);
}

// ---------------------------------------------------------------------------
// D1. hello -> ack (control plane is live, versioned)
// ---------------------------------------------------------------------------

#[test]
fn hello_gets_ack() {
    let topic = unique_topic("phase_d_hello");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic,
        ..Default::default()
    })
    .unwrap();
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    client
        .send_control(&Control::Hello {
            proto: PROTO_VERSION,
            client: "t".into(),
        })
        .unwrap();
    expect_ack(&mut client, 0);
}

// ---------------------------------------------------------------------------
// D2. one gateway, two topics: both directions, tagged per lane
// ---------------------------------------------------------------------------

#[test]
fn multi_topic_routing_both_directions() {
    let a = unique_topic("phase_d_a");
    let b = unique_topic("phase_d_b");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: a.clone(),
        extra_topics: vec![b.clone()],
        ..Default::default()
    })
    .unwrap();
    let peer_a = make_peer(&a);
    let peer_b = make_peer(&b);
    std::thread::sleep(Duration::from_millis(800));

    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    let ma = sample(1);
    let mb = sample(2);
    client.send_echo(&a, &ma).unwrap();
    client.send_echo(&b, &mb).unwrap();
    assert!(
        wait_peer(&peer_a.reader, &ma, Duration::from_secs(10)),
        "lane A up"
    );
    assert!(
        wait_peer(&peer_b.reader, &mb, Duration::from_secs(10)),
        "lane B up"
    );

    // Downstream: both lanes loop back, each tagged with its own topic.
    let mut got = Vec::new();
    let mut errors = Vec::new();
    for _ in 0..2 {
        let (t, m, _) = client
            .recv_echo(Duration::from_secs(10), &mut errors)
            .unwrap();
        got.push((t, m));
    }
    got.sort_by(|x, y| x.0.cmp(&y.0));
    let mut want = vec![(a.clone(), ma), (b.clone(), mb)];
    want.sort_by(|x, y| x.0.cmp(&y.0));
    assert_eq!(got, want);
    assert!(errors.is_empty(), "{errors:?}");
}

// ---------------------------------------------------------------------------
// D3. per-connection subscriptions filter downstream
// ---------------------------------------------------------------------------

#[test]
fn unsubscribe_filters_downstream_then_resubscribe_restores() {
    let a = unique_topic("phase_d_sub_a");
    let b = unique_topic("phase_d_sub_b");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: a.clone(),
        extra_topics: vec![b.clone()],
        ..Default::default()
    })
    .unwrap();
    let peer_b = make_peer(&b);
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    std::thread::sleep(Duration::from_millis(800));

    client
        .send_control(&Control::Unsubscribe {
            proto: PROTO_VERSION,
            topic: b.clone(),
            seq: 1,
        })
        .unwrap();
    expect_ack(&mut client, 1);

    // B traffic exists on DDS but this connection unsubscribed: nothing arrives.
    let mb = sample(21);
    peer_b
        .writer
        .write(&WasmEcho::from_proto(&mb).unwrap())
        .unwrap();
    let mut errors = Vec::new();
    let err = client
        .recv_echo(Duration::from_millis(1200), &mut errors)
        .expect_err("B must be filtered");
    assert!(matches!(err, BridgeError::Timeout), "got {err}");

    // Resubscribe: flow resumes on the same connection.
    client
        .send_control(&Control::Subscribe {
            proto: PROTO_VERSION,
            topic: b.clone(),
            seq: 2,
        })
        .unwrap();
    expect_ack(&mut client, 2);
    let mb2 = sample(22);
    peer_b
        .writer
        .write(&WasmEcho::from_proto(&mb2).unwrap())
        .unwrap();
    let (t, back, _) = client
        .recv_echo(Duration::from_secs(10), &mut errors)
        .unwrap();
    assert_eq!((t.as_str(), back), (b.as_str(), mb2));

    // Unknown topic subscribe fails loud, connection survives.
    client
        .send_control(&Control::Subscribe {
            proto: PROTO_VERSION,
            topic: "nope".into(),
            seq: 3,
        })
        .unwrap();
    expect_error(&mut client, "unknown_topic");
    let _ = (a, bridge);
}

// ---------------------------------------------------------------------------
// D4. register QoS: compatible -> ack, reliable/transient-local -> loud
// ---------------------------------------------------------------------------

#[test]
fn register_qos_compatible_acks_incompatible_fails_loud() {
    let topic = unique_topic("phase_d_reg");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: topic.clone(),
        ..Default::default()
    })
    .unwrap();
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();

    client
        .send_control(&Control::Register {
            proto: PROTO_VERSION,
            topic: topic.clone(),
            type_name: cyclonedds_proto::echo::ECHO_TYPE.into(),
            qos: Qos::default(),
            seq: 7,
        })
        .unwrap();
    expect_ack(&mut client, 7);

    for (seq, qos) in [
        (
            8,
            Qos {
                reliability: Reliability::Reliable,
                durability: Durability::Volatile,
            },
        ),
        (
            9,
            Qos {
                reliability: Reliability::BestEffort,
                durability: Durability::TransientLocal,
            },
        ),
    ] {
        client
            .send_control(&Control::Register {
                proto: PROTO_VERSION,
                topic: topic.clone(),
                type_name: cyclonedds_proto::echo::ECHO_TYPE.into(),
                qos,
                seq,
            })
            .unwrap();
        expect_error(&mut client, "unsupported_qos");
    }
    // Stats heard the loud failures; the connection still works.
    assert!(bridge.stats().errors_out >= 2, "{:?}", bridge.stats());
    let m = sample(30);
    client.send_echo(&topic, &m).unwrap();
    let mut errors = Vec::new();
    let (_, back, _) = client
        .recv_echo(Duration::from_secs(10), &mut errors)
        .unwrap();
    assert_eq!(back, m);
}

// ---------------------------------------------------------------------------
// D5. backoff schedule is pure + reconnect budget fails loud
// ---------------------------------------------------------------------------

#[test]
fn backoff_schedule_doubles_and_caps() {
    let base = Duration::from_millis(50);
    let cap = Duration::from_millis(200);
    assert_eq!(backoff_delay(0, base, cap), Duration::from_millis(50));
    assert_eq!(backoff_delay(1, base, cap), Duration::from_millis(100));
    assert_eq!(backoff_delay(2, base, cap), Duration::from_millis(200));
    assert_eq!(backoff_delay(9, base, cap), Duration::from_millis(200));
    assert_eq!(
        backoff_delay(u32::MAX, base, cap),
        cap,
        "saturating, never hangs"
    );
}

#[test]
fn connect_retry_exhausts_budget_loudly() {
    // A port that was bound and released: (almost surely) nobody listens.
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = probe.local_addr().unwrap();
    drop(probe);
    let start = Instant::now();
    let err = BridgeClient::connect_with_retry(
        addr,
        2,
        Duration::from_millis(50),
        Duration::from_millis(100),
    )
    .expect_err("closed port must fail");
    assert!(matches!(err, BridgeError::Io(_)), "got {err}");
    // Budget actually slept: 50 + 100 ms, well under a hang.
    assert!(
        start.elapsed() >= Duration::from_millis(140),
        "no backoff slept?"
    );
    assert!(start.elapsed() < Duration::from_secs(10));
}

#[test]
fn connect_retry_joins_listener_that_appears_late() {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = probe.local_addr().unwrap();
    drop(probe);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(150));
        // Best effort: if the OS has not recycled the port yet, this bind
        // fails and the client below times out on its own budget instead of
        // hanging — either way the test asserts an observable outcome.
        if let Ok(late) = std::net::TcpListener::bind(addr) {
            let _ = late.accept();
        }
    });
    let res = BridgeClient::connect_with_retry(
        addr,
        30,
        Duration::from_millis(20),
        Duration::from_millis(50),
    );
    assert!(res.is_ok(), "late listener not joined: {:?}", res.err());
}

// ---------------------------------------------------------------------------
// D6. cancellation wakes a quiet wait; quiet wait without cancel times out
// ---------------------------------------------------------------------------

#[test]
fn cancel_wakes_quiet_wait() {
    let topic = unique_topic("phase_d_cancel");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic,
        ..Default::default()
    })
    .unwrap();
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    let flag = CancelFlag::new();
    let flag2 = flag.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        flag2.cancel();
    });
    let mut errors = Vec::new();
    let start = Instant::now();
    let err = client
        .recv_echo_cancel(Duration::from_secs(30), &mut errors, &flag)
        .expect_err("cancel must win over the deadline");
    assert!(matches!(err, BridgeError::Cancelled), "got {err}");
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "cancel did not wake the wait"
    );
}

#[test]
fn quiet_wait_without_cancel_times_out() {
    let topic = unique_topic("phase_d_qwait");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic,
        ..Default::default()
    })
    .unwrap();
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    let mut errors = Vec::new();
    let err = client
        .recv_echo_cancel(Duration::from_millis(400), &mut errors, &CancelFlag::new())
        .expect_err("quiet wait must time out");
    assert!(matches!(err, BridgeError::Timeout), "got {err}");
}

// ---------------------------------------------------------------------------
// D7. bounded queue: a stuck consumer produces explicit overflow, and the
// DDS owner thread never stalls on it
// ---------------------------------------------------------------------------

#[test]
fn stuck_consumer_overflows_explicitly_without_stalling_dds() {
    let topic = unique_topic("phase_d_overflow");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: topic.clone(),
        queue_cap: 1,
        ..Default::default()
    })
    .unwrap();
    let peer = make_peer(&topic);
    std::thread::sleep(Duration::from_millis(800));

    // Connect and never read: kernel buffers fill, the pump blocks, the
    // bounded queue fills, further frames drop with accounting.
    let _stuck = BridgeClient::connect(bridge.addr()).unwrap();
    let big = EchoMsg {
        id: 0,
        text: "X".repeat(32 * 1024),
        values: vec![1, 2, 3],
    };
    for i in 0..300 {
        let mut m = big.clone();
        m.id = i;
        peer.writer
            .write(&WasmEcho::from_proto(&m).unwrap())
            .unwrap();
    }
    let start = Instant::now();
    loop {
        if bridge.stats().dropped_overflow >= 1 {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "no overflow ever counted: {:?}",
            bridge.stats()
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    // The owner thread kept pumping while overflowing (monotone progress).
    let s = bridge.stats();
    assert!(s.samples_out >= 1, "{s:?}");
    assert!(s.dropped_overflow >= 1, "{s:?}");
}

// ---------------------------------------------------------------------------
// D8. close + drop: functional roundtrip before and after, no poisoning
// ---------------------------------------------------------------------------

#[test]
fn close_and_drop_leaves_no_poisoned_state() {
    let topic = unique_topic("phase_d_drop");
    ensure_loopback_dds();
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: topic.clone(),
        ..Default::default()
    })
    .unwrap();
    let peer = make_peer(&topic);
    std::thread::sleep(Duration::from_millis(800));
    let mut client = BridgeClient::connect(bridge.addr()).unwrap();
    let m = sample(40);
    client.send_echo(&topic, &m).unwrap();
    assert!(wait_peer(&peer.reader, &m, Duration::from_secs(10)));
    bridge.close();
    bridge.close(); // idempotent
    drop(bridge);
    drop(client);
    drop(peer);

    // A fresh gateway on the same process state works immediately.
    let topic2 = unique_topic("phase_d_drop2");
    let bridge2 = WasmBridge::bind(BridgeConfig {
        topic: topic2.clone(),
        ..Default::default()
    })
    .unwrap();
    let peer2 = make_peer(&topic2);
    std::thread::sleep(Duration::from_millis(800));
    let mut client2 = BridgeClient::connect(bridge2.addr()).unwrap();
    let m2 = sample(41);
    client2.send_echo(&topic2, &m2).unwrap();
    assert!(
        wait_peer(&peer2.reader, &m2, Duration::from_secs(10)),
        "rebind after drop failed"
    );
}
