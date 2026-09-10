//! Shutdown-under-saturation soak: floods a live gateway and closes it
//! mid-burst, over and over, on both data planes.
//!
//! Why this exists: the dispatcher has two arms that only run when the
//! DDS owner thread is already gone while a connection thread still has
//! a fully-read packet in hand (`publish_to_dds → false`: legacy Bye,
//! binary break). The owner thread has no panic path and exits only via
//! `close()`, so the only trigger is the close race itself. Saturating
//! every connection with large packets keeps all dispatchers inside the
//! read→publish window (milliseconds per packet) while `close()` drops
//! the owner underneath them, which makes the race fire in practice.
//!
//! The test itself asserts only deterministic invariants (every close
//! returns, stats stay readable, a fresh gateway serves afterwards): a
//! missed race never fails the test, it only shows up in coverage.

use cyclonedds_proto::echo::{encode_echo_xcdr1, EchoMsg};
use cyclonedds_proto::{DataFrame, FLAG_CDR_LE, PROTO_VERSION};
use dds_wasm_e2e::*;
use std::io::Write;
use std::net::TcpStream;

const CONNS: usize = 32;
const FEED_EACH: usize = 10;

fn legacy_packet(topic: &str, id: i32, values: usize) -> Vec<u8> {
    let mut s =
        format!("{{\"topic\":{topic:?},\"data\":{{\"id\":{id},\"text\":\"soak\",\"values\":[");
    for i in 0..values {
        if i > 0 {
            s.push(',');
        }
        s.push_str("1234567");
    }
    s.push_str("]}}");
    let mut out = (s.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out
}

fn binary_packet(topic: &str, seq: u32, values: usize) -> Vec<u8> {
    let cdr = encode_echo_xcdr1(&EchoMsg {
        id: seq as i32,
        text: "soak".into(),
        values: vec![7; values],
    })
    .unwrap();
    let frame = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: topic.into(),
        cdr,
        seq,
    }
    .encode()
    .unwrap();
    let mut out = (frame.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(&frame);
    out
}

/// Flood `CONNS` connections with `make_packet` bytes, then close
/// mid-burst. Returns after the close; connection threads may still be
/// draining, which is exactly the state under test.
fn flood_and_close(topic: &str, legacy: bool) {
    let bridge = if legacy {
        bind_gateway_legacy(topic)
    } else {
        bind_gateway(topic)
    };
    let mut socks: Vec<TcpStream> = (0..CONNS)
        .map(|_| {
            let s = TcpStream::connect(bridge.addr()).expect("soak connect");
            // A burst that outlives the gateway must fail fast, never hang:
            // after `close()` a feeder can sit in write_all on a CLOSE_WAIT
            // socket with full buffers (FIN interrupts reads, not writes).
            s.set_write_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("write timeout");
            s
        })
        .collect();
    let feeders: Vec<std::thread::JoinHandle<()>> = socks
        .iter_mut()
        .map(|sock| {
            let mut sock = sock.try_clone().expect("feeder clone");
            let topic = topic.to_string();
            std::thread::spawn(move || {
                for i in 0..FEED_EACH {
                    let pkt = if legacy {
                        legacy_packet(&topic, i as i32, 30_000)
                    } else {
                        binary_packet(&topic, i as u32, 50_000)
                    };
                    // The burst outlives the gateway: ignore post-close errors.
                    let _ = sock.write_all(&pkt);
                }
            })
        })
        .collect();
    // Precondition with a deadline: the burst must really publish before we
    // close. On a loaded runner 50ms may see only arrivals (frames_in > 0,
    // samples_in == 0) because close() wins the race before the first
    // publish lands. The race under test needs in-flight publishes when
    // close() drops the owner, so wait for the first publish, then close
    // while the feeders (32 conns x 10 x ~200KB) still saturate everything.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while bridge.stats().samples_in == 0 {
        if std::time::Instant::now() > deadline {
            panic!("burst never published: {:?}", bridge.stats());
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    bridge.close();
    for f in feeders {
        let _ = f.join();
    }
    // Deterministic invariants: close is terminal but observable, and
    // the burst really published (not just arrived): the race under test
    // needs in-flight publishes, and a silently-rejected burst would hide
    // that precondition while still counting frames.
    let stats = bridge.stats();
    assert!(stats.frames_in > 0, "burst must have been served");
    assert!(stats.samples_in > 0, "burst must have published: {stats:?}");
    drop(socks);
}

#[test]
fn shutdown_during_saturation_is_clean_on_both_planes() {
    for round in 0..5 {
        flood_and_close(&unique_topic(&format!("soak_leg{round}")), true);
    }
    for round in 0..5 {
        flood_and_close(&unique_topic(&format!("soak_bin{round}")), false);
    }
    // No global state corruption: a fresh gateway serves end to end.
    let topic = unique_topic("soak_after");
    let bridge = bind_gateway(&topic);
    let mut client = connect(&bridge);
    client.send_echo(&topic, &sample(1)).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while bridge.stats().samples_in != 1 {
        assert!(std::time::Instant::now() < deadline, "gateway stalled");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
