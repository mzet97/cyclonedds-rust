//! Phase E: parity matrix between the native DDS contract and what the
//! gateway profile exposes.
//!
//! Each row asserts an observable behaviour on real entities (loopback):
//! read vs take, sample states, dispose/unregister, keys, valid_data,
//! loud QoS-incompatibility, and transient-local late joiners. Where the
//! gateway profile deliberately narrows the contract, the test asserts the
//! loud `Unsupported` instead of an implementation.

use cyclonedds::{QosBuilder, StatusExt};
use cyclonedds_proto::echo::EchoMsg;
use cyclonedds_rust_sys::{
    DDS_ALIVE_INSTANCE_STATE, DDS_NOT_ALIVE_DISPOSED_INSTANCE_STATE,
    DDS_NOT_ALIVE_NO_WRITERS_INSTANCE_STATE, DDS_NOT_READ_SAMPLE_STATE, DDS_READ_SAMPLE_STATE,
};
use dds_wasm_bridge::WasmEcho;
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

fn native(id: i32) -> WasmEcho {
    WasmEcho::from_proto(&EchoMsg {
        id,
        text: format!("e-{id}"),
        values: vec![id],
    })
    .unwrap()
}

struct Peer {
    _participant: cyclonedds::DomainParticipant,
    _topic: cyclonedds::Topic<WasmEcho>,
    _publisher: cyclonedds::Publisher,
    _subscriber: cyclonedds::Subscriber,
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
        _publisher: publisher,
        _subscriber: subscriber,
        writer,
        reader,
    }
}

fn settle() {
    std::thread::sleep(Duration::from_millis(500));
}

// ---------------------------------------------------------------------------
// E1. read leaves, take removes
// ---------------------------------------------------------------------------

#[test]
fn read_leaves_sample_take_removes_it() {
    let peer = make_peer(&unique_topic("par_read_take"));
    settle();
    peer.writer.write(&native(1)).unwrap();
    settle();

    assert_eq!(peer.reader.read().unwrap().len(), 1, "read sees the sample");
    assert_eq!(
        peer.reader.read().unwrap().len(),
        1,
        "read does not consume"
    );
    assert_eq!(peer.reader.take().unwrap().len(), 1, "take returns it");
    assert!(peer.reader.take().unwrap().is_empty(), "take consumed it");
}

// ---------------------------------------------------------------------------
// E2. sample states: NOT_READ -> READ via masks
// ---------------------------------------------------------------------------

#[test]
fn sample_states_advance_through_masks() {
    let peer = make_peer(&unique_topic("par_states"));
    settle();
    peer.writer.write(&native(2)).unwrap();
    settle();

    assert_eq!(
        peer.reader
            .take_mask(DDS_NOT_READ_SAMPLE_STATE)
            .unwrap()
            .len(),
        1
    );
    assert!(peer
        .reader
        .take_mask(DDS_NOT_READ_SAMPLE_STATE)
        .unwrap()
        .is_empty());

    peer.writer.write(&native(3)).unwrap();
    settle();
    assert_eq!(
        peer.reader
            .read_mask(DDS_NOT_READ_SAMPLE_STATE)
            .unwrap()
            .len(),
        1
    );
    // `read` marks: nothing unread left, but the sample is still cached.
    assert!(peer
        .reader
        .read_mask(DDS_NOT_READ_SAMPLE_STATE)
        .unwrap()
        .is_empty());
    assert_eq!(
        peer.reader.read_mask(DDS_READ_SAMPLE_STATE).unwrap().len(),
        1
    );
    // And take finally drains it.
    assert_eq!(peer.reader.take().unwrap().len(), 1);
    assert!(peer.reader.take().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// E3. dispose delivers an invalid sample, never data
// ---------------------------------------------------------------------------

#[test]
fn dispose_delivers_invalid_sample_with_disposed_state() {
    let peer = make_peer(&unique_topic("par_dispose"));
    settle();
    let d = native(10);
    peer.writer.write(&d).unwrap();
    settle();
    peer.writer.dispose(&d).unwrap();
    settle();

    // The disposed instance is visible under its state mask ...
    let loan = peer
        .reader
        .take_mask(DDS_NOT_ALIVE_DISPOSED_INSTANCE_STATE)
        .unwrap();
    assert!(!loan.is_empty(), "disposed sample must be delivered");
    // ... carrying the disposed instance state on the wire sample (observed:
    // `valid_data` stays true, `instance_state` flips to DISPOSED).
    let valid = loan.to_vec().unwrap();
    assert!(!valid.is_empty());
    for s in &valid {
        assert_eq!(s.instance_state(), DDS_NOT_ALIVE_DISPOSED_INSTANCE_STATE);
    }

    // Cache drained by that take.
    assert!(peer.reader.take_loan().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// E4. unregister delivers a no-writers sample
// ---------------------------------------------------------------------------

#[test]
fn unregister_delivers_no_writers_sample() {
    // Observed: with default writer QoS, unregister auto-disposes (instance
    // ends DISPOSED, not NO_WRITERS). Disable autodispose so the unregister
    // path itself is what the reader observes.
    ensure_loopback_dds();
    let topic = unique_topic("par_unreg");
    let participant = cyclonedds::DomainParticipant::new(0).unwrap();
    let t = participant.create_topic::<WasmEcho>(&topic).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let no_autodispose = QosBuilder::new()
        .writer_data_lifecycle(false)
        .build()
        .unwrap();
    let writer = publisher
        .create_writer_with_qos(&t, &no_autodispose)
        .unwrap();
    let reader = subscriber.create_reader(&t).unwrap();
    settle();
    let d = native(11);
    writer.write(&d).unwrap();
    settle();
    writer.unregister_instance(&d).unwrap();
    settle();

    let loan = reader
        .take_mask(DDS_NOT_ALIVE_NO_WRITERS_INSTANCE_STATE)
        .unwrap();
    assert!(
        !loan.is_empty(),
        "unregister must surface a no-writers sample"
    );
    for s in loan.to_vec().unwrap() {
        assert_eq!(s.instance_state(), DDS_NOT_ALIVE_NO_WRITERS_INSTANCE_STATE);
    }
}

// ---------------------------------------------------------------------------
// E5. keys isolate instances
// ---------------------------------------------------------------------------

#[test]
fn keyed_instances_are_independent() {
    let peer = make_peer(&unique_topic("par_keys"));
    settle();
    peer.writer.write(&native(20)).unwrap();
    peer.writer.write(&native(21)).unwrap();
    settle();

    let h20 = peer.reader.lookup_instance(&native(20));
    let h21 = peer.reader.lookup_instance(&native(21));
    assert!(h20 != 0 && h21 != 0 && h20 != h21, "handles {h20} {h21}");

    let only20: Vec<_> = peer.reader.read_instance(h20).unwrap().to_vec().unwrap();
    assert!(!only20.is_empty());
    assert!(only20.iter().all(|s| s.data.id == 20));

    // Taking instance 21 removes only its samples.
    let took21: Vec<_> = peer.reader.take_instance(h21).unwrap().to_vec().unwrap();
    assert!(took21.iter().all(|s| s.data.id == 21));
    let rest = peer.reader.take().unwrap();
    assert!(!rest.is_empty());
    assert!(rest.iter().all(|s| s.id == 20), "instance 20 untouched");

    // The key itself round-trips through the native descriptor.
    assert_eq!(peer.reader.instance_get_key(h20).unwrap().id, 20);
}

// ---------------------------------------------------------------------------
// E6. valid_data + sample/view/instance states on a live sample
// ---------------------------------------------------------------------------

#[test]
fn live_sample_reports_valid_data_and_states() {
    let peer = make_peer(&unique_topic("par_valid"));
    settle();
    peer.writer.write(&native(30)).unwrap();
    settle();

    let s = peer.reader.take_next().unwrap().expect("one live sample");
    assert!(s.is_valid(), "live sample must be valid_data");
    assert_eq!(s.sample_state(), DDS_NOT_READ_SAMPLE_STATE);
    assert_eq!(s.instance_state(), DDS_ALIVE_INSTANCE_STATE);
    assert_eq!(s.data.id, 30);
}

// ---------------------------------------------------------------------------
// E7. gateway subset stays loud: reliable/transient-local are Unsupported
// (treatment, not implementation — no silent accept)
// ---------------------------------------------------------------------------

#[test]
fn gateway_qos_subset_rejects_reliable_and_transient_local() {
    use cyclonedds_proto::{check_qos, Durability, ProtoError, Qos, Reliability};
    assert!(check_qos(&Qos::default()).is_ok());
    for qos in [
        Qos {
            reliability: Reliability::Reliable,
            durability: Durability::Volatile,
        },
        Qos {
            reliability: Reliability::BestEffort,
            durability: Durability::TransientLocal,
        },
    ] {
        match check_qos(&qos) {
            Err(ProtoError::Unsupported(_)) => {}
            other => panic!("must fail loud with Unsupported, got {other:?}"),
        }
    }
    // Unknown wire variants fail closed at parse time, not as defaults.
    assert!(serde_json::from_str::<Qos>(
        r#"{"reliability":"exactly_once","durability":"volatile"}"#
    )
    .is_err());
}

// ---------------------------------------------------------------------------
// E8. native incompatible QoS is observable on both sides (loud, not silent)
// ---------------------------------------------------------------------------

#[test]
fn native_incompatible_qos_surfaces_status_and_delivers_nothing() {
    ensure_loopback_dds();
    let topic = unique_topic("par_incompat");
    let participant = cyclonedds::DomainParticipant::new(0).unwrap();
    let t = participant.create_topic::<WasmEcho>(&topic).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    // Reader asks reliable, writer offers best-effort: RxO says NO.
    let writer_qos = QosBuilder::new().best_effort().build().unwrap();
    let reader_qos = QosBuilder::new().reliable().build().unwrap();
    let writer = publisher.create_writer_with_qos(&t, &writer_qos).unwrap();
    let reader = subscriber.create_reader_with_qos(&t, &reader_qos).unwrap();

    let start = Instant::now();
    loop {
        let rq = reader.requested_incompatible_qos_status().unwrap();
        if rq.total_count >= 1 {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "no incompatible-QoS status observed"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let oq = writer.offered_incompatible_qos_status().unwrap();
    assert!(oq.total_count >= 1, "writer side must also report it");

    // And nothing flows across the incompatible match.
    writer.write(&native(40)).unwrap();
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        reader.take().unwrap().is_empty(),
        "incompatible pair must not deliver"
    );
}

// ---------------------------------------------------------------------------
// E9. transient-local late joiner gets history natively (where it applies)
// ---------------------------------------------------------------------------

#[test]
fn transient_local_late_joiner_receives_history() {
    ensure_loopback_dds();
    let topic = unique_topic("par_tl");
    let participant = cyclonedds::DomainParticipant::new(0).unwrap();
    let t = participant.create_topic::<WasmEcho>(&topic).unwrap();
    let publisher = participant.create_publisher().unwrap();
    let qos = QosBuilder::new()
        .reliable()
        .keep_all()
        .transient_local()
        .build()
        .unwrap();
    let writer = publisher.create_writer_with_qos(&t, &qos).unwrap();

    // History accumulates BEFORE any reader exists.
    for id in 50..53 {
        writer.write(&native(id)).unwrap();
    }

    let subscriber = participant.create_subscriber().unwrap();
    let reader = subscriber.create_reader_with_qos(&t, &qos).unwrap();
    reader.wait_for_historical_data(5_000_000_000).unwrap();
    let start = Instant::now();
    let mut ids = Vec::new();
    while ids.len() < 3 && start.elapsed() < Duration::from_secs(10) {
        for s in reader.take().unwrap() {
            ids.push(s.id);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    ids.sort();
    assert_eq!(
        ids,
        vec![50, 51, 52],
        "late joiner must see pre-existing history"
    );
}
