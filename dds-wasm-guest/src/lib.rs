//! WASI guest-side dispatcher (Phase F): the untrusted half of
//! `tese:dds-bridge@0.1.0` (`wit/dds-bridge.wit`).
//!
//! Portable by construction: depends only on `cyclonedds-proto` (proto v0
//! codec + control plane). No `cyclonedds-rust-sys`, no `wasm-bindgen`, no
//! sockets, no threads. The host (native gateway) owns every DDS entity;
//! this side only encodes/decodes frames, tracks its own subscriptions,
//! and bounds its outbox with explicit overflow (`drops`, never silent,
//! never blocking).
//!
//! [`WitEchoSample`] is the Rust projection of the WIT `echo-sample`
//! record: field-for-field (`id: s32`, `text: string`, `values:
//! list<s32>`). Changing the WIT record without updating this struct (or
//! vice versa) must bump the package version.

use cyclonedds_proto::{
    check_qos,
    echo::{decode_echo_xcdr1, encode_echo_xcdr1, EchoMsg},
    Control, DataFrame, ProtoError, FLAG_CDR_LE, PROTO_VERSION,
};
use std::collections::{HashSet, VecDeque};

/// Rust projection of WIT `tese:dds-bridge@0.1.0/types.echo-sample`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitEchoSample {
    pub id: i32,
    pub text: String,
    pub values: Vec<i32>,
}

impl From<&EchoMsg> for WitEchoSample {
    fn from(m: &EchoMsg) -> Self {
        WitEchoSample {
            id: m.id,
            text: m.text.clone(),
            values: m.values.clone(),
        }
    }
}

impl From<&WitEchoSample> for EchoMsg {
    fn from(w: &WitEchoSample) -> Self {
        EchoMsg {
            id: w.id,
            text: w.text.clone(),
            values: w.values.clone(),
        }
    }
}

/// Guest-side failure. Every variant maps 1:1 to WIT `guest-error`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestError {
    InvalidFrame(String),
    UnknownTopic(String),
    UnsupportedQos(String),
    UnsupportedProto(String),
    QueueFull,
    NotConnected,
}

impl std::fmt::Display for GuestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuestError::InvalidFrame(s) => write!(f, "invalid frame: {s}"),
            GuestError::UnknownTopic(s) => write!(f, "unknown topic: {s}"),
            GuestError::UnsupportedQos(s) => write!(f, "unsupported qos: {s}"),
            GuestError::UnsupportedProto(s) => write!(f, "unsupported proto: {s}"),
            GuestError::QueueFull => write!(f, "guest outbox full"),
            GuestError::NotConnected => write!(f, "not connected"),
        }
    }
}

impl std::error::Error for GuestError {}

impl From<ProtoError> for GuestError {
    fn from(e: ProtoError) -> Self {
        match e {
            ProtoError::Unsupported(s) => GuestError::UnsupportedProto(s),
            ProtoError::Serialization(s) => GuestError::InvalidFrame(s),
            ProtoError::TooLarge { got, max } => {
                GuestError::InvalidFrame(format!("frame too large: {got} (max {max})"))
            }
            ProtoError::InvalidFrame(s) => GuestError::InvalidFrame(s.to_string()),
            ProtoError::NotConnected => GuestError::NotConnected,
        }
    }
}

/// Guest dispatcher: WIT `guest` world in portable Rust.
pub struct GuestDispatcher {
    /// Topics the host may deliver (mirrors host-side subscriptions).
    subs: HashSet<String>,
    /// Outbound encoded frames waiting for the host to collect.
    outbox: VecDeque<Vec<u8>>,
    outbox_cap: usize,
    /// Frames dropped because the outbox was full. Explicit overflow.
    drops: u64,
    /// Inbound gap detector per topic.
    last_seq: std::collections::HashMap<String, u32>,
    pub seq_gaps: u64,
    next_seq: u32,
}

impl GuestDispatcher {
    pub fn new(topics: &[&str], outbox_cap: usize) -> Self {
        GuestDispatcher {
            subs: topics.iter().map(|s| s.to_string()).collect(),
            outbox: VecDeque::new(),
            outbox_cap: outbox_cap.max(1),
            drops: 0,
            last_seq: std::collections::HashMap::new(),
            seq_gaps: 0,
            next_seq: 0,
        }
    }

    /// WIT `encode-frame`: encode + assign `seq`, stage into the bounded
    /// outbox. `QueueFull` drops with accounting (never grows, never blocks).
    pub fn encode_frame(&mut self, topic: &str, sample: &WitEchoSample) -> Result<(), GuestError> {
        let msg = EchoMsg::from(sample);
        let cdr = encode_echo_xcdr1(&msg)?;
        let frame = DataFrame {
            proto: PROTO_VERSION,
            flags: FLAG_CDR_LE,
            topic: topic.into(),
            cdr,
            seq: self.next_seq,
        };
        self.next_seq = self.next_seq.wrapping_add(1);
        let bytes = frame.encode()?;
        if self.outbox.len() >= self.outbox_cap {
            self.drops += 1;
            return Err(GuestError::QueueFull);
        }
        self.outbox.push_back(bytes);
        Ok(())
    }

    /// WIT `handle-frame`: decode one inbound frame. Unsubscribed topics
    /// return `Ok(None)` (filtered, never an error); gaps are counted.
    pub fn handle_frame(
        &mut self,
        bytes: &[u8],
    ) -> Result<Option<(String, WitEchoSample, u32)>, GuestError> {
        let frame = DataFrame::decode(bytes)?;
        if frame.flags != FLAG_CDR_LE {
            return Err(GuestError::InvalidFrame(format!(
                "echo data plane requires CDR-LE, flags {:#06x}",
                frame.flags
            )));
        }
        if !self.subs.contains(&frame.topic) {
            return Ok(None);
        }
        match self.last_seq.get(&frame.topic) {
            Some(prev) if frame.seq != prev.wrapping_add(1) => self.seq_gaps += 1,
            _ => {}
        }
        self.last_seq.insert(frame.topic.clone(), frame.seq);
        let msg = decode_echo_xcdr1(&frame.cdr)?;
        Ok(Some((frame.topic, WitEchoSample::from(&msg), frame.seq)))
    }

    /// WIT `handle-control`: one control JSON in, reply JSON out (if any).
    /// Unknown topics and non-subset QoS fail loud with typed errors.
    pub fn handle_control(&mut self, json: &str) -> Result<Option<String>, GuestError> {
        let ctrl = Control::from_json(json).map_err(|e| GuestError::InvalidFrame(e.to_string()))?;
        ctrl.check_version()?;
        match ctrl {
            Control::Hello { .. } => Ok(Some(
                Control::Ack {
                    proto: PROTO_VERSION,
                    seq: 0,
                }
                .to_json()?,
            )),
            Control::Subscribe { topic, seq, .. } => {
                self.subs.insert(topic);
                Ok(Some(
                    Control::Ack {
                        proto: PROTO_VERSION,
                        seq,
                    }
                    .to_json()?,
                ))
            }
            Control::Unsubscribe { topic, seq, .. } => {
                self.subs.remove(&topic);
                Ok(Some(
                    Control::Ack {
                        proto: PROTO_VERSION,
                        seq,
                    }
                    .to_json()?,
                ))
            }
            Control::Register {
                topic, qos, seq, ..
            } => {
                check_qos(&qos).map_err(|e| GuestError::UnsupportedQos(e.to_string()))?;
                self.subs.insert(topic);
                Ok(Some(
                    Control::Ack {
                        proto: PROTO_VERSION,
                        seq,
                    }
                    .to_json()?,
                ))
            }
            Control::Bye { .. } | Control::Ack { .. } | Control::Error { .. } => Ok(None),
        }
    }

    /// WIT `outbox-stats`: `(depth, lifetime_drops)`.
    pub fn outbox_stats(&self) -> (u32, u64) {
        (self.outbox.len() as u32, self.drops)
    }

    /// Host collects staged outbound frames (drains the outbox).
    pub fn drain_outbox(&mut self) -> Vec<Vec<u8>> {
        self.outbox.drain(..).collect()
    }

    pub fn subscriptions(&self) -> Vec<String> {
        let mut out: Vec<_> = self.subs.iter().cloned().collect();
        out.sort();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cyclonedds_proto::{Durability, Qos, Reliability};

    fn sample(id: i32) -> WitEchoSample {
        WitEchoSample {
            id,
            text: format!("w-{id}"),
            values: vec![id, id + 1],
        }
    }

    #[test]
    fn wit_record_roundtrips_through_cdr_frames() {
        let mut g = GuestDispatcher::new(&["T"], 8);
        let s = sample(5);
        g.encode_frame("T", &s).unwrap();
        let staged = g.drain_outbox();
        assert_eq!(staged.len(), 1);
        let (t, back, seq) = g.handle_frame(&staged[0]).unwrap().expect("subscribed");
        assert_eq!((t.as_str(), back, seq), ("T", s, 0));
    }

    #[test]
    fn guest_filters_unsubscribed_and_counts_gaps() {
        let mut g = GuestDispatcher::new(&["A"], 8);
        g.encode_frame("B", &sample(1)).unwrap();
        let staged = g.drain_outbox();
        assert!(
            g.handle_frame(&staged[0]).unwrap().is_none(),
            "B not subscribed"
        );

        // Gap: seq 0 then seq 5 on a subscribed topic.
        let mk = |seq: u32| {
            let cdr = encode_echo_xcdr1(&EchoMsg::from(&sample(2))).unwrap();
            DataFrame {
                proto: PROTO_VERSION,
                flags: FLAG_CDR_LE,
                topic: "A".into(),
                cdr,
                seq,
            }
            .encode()
            .unwrap()
        };
        g.handle_frame(&mk(0)).unwrap();
        g.handle_frame(&mk(5)).unwrap();
        assert_eq!(g.seq_gaps, 1);
    }

    #[test]
    fn outbox_overflow_is_explicit() {
        let mut g = GuestDispatcher::new(&["T"], 2);
        g.encode_frame("T", &sample(1)).unwrap();
        g.encode_frame("T", &sample(2)).unwrap();
        let err = g.encode_frame("T", &sample(3)).expect_err("outbox cap 2");
        assert_eq!(err, GuestError::QueueFull);
        assert_eq!(g.outbox_stats(), (2, 1));
    }

    #[test]
    fn control_plane_subscribes_and_rejects_loudly() {
        let mut g = GuestDispatcher::new(&[], 8);
        let hello = Control::Hello {
            proto: PROTO_VERSION,
            client: "w".into(),
        }
        .to_json()
        .unwrap();
        let reply = g.handle_control(&hello).unwrap().expect("ack");
        assert!(matches!(
            Control::from_json(&reply).unwrap(),
            Control::Ack { .. }
        ));

        let sub = Control::Subscribe {
            proto: PROTO_VERSION,
            topic: "T".into(),
            seq: 3,
        }
        .to_json()
        .unwrap();
        g.handle_control(&sub).unwrap();
        assert_eq!(g.subscriptions(), vec!["T".to_string()]);

        let bad_qos = Control::Register {
            proto: PROTO_VERSION,
            topic: "T".into(),
            type_name: "WasmEcho".into(),
            qos: Qos {
                reliability: Reliability::Reliable,
                durability: Durability::Volatile,
            },
            seq: 4,
        }
        .to_json()
        .unwrap();
        assert_eq!(
            g.handle_control(&bad_qos).expect_err("reliable must fail"),
            GuestError::UnsupportedQos("unsupported: reliability != best_effort".into())
        );

        let future = r#"{"proto":99,"kind":"hello","client":"w"}"#;
        assert!(matches!(
            g.handle_control(future).expect_err("proto 99"),
            GuestError::UnsupportedProto(_)
        ));
        assert!(g
            .handle_control("not json")
            .expect_err("garbage")
            .to_string()
            .contains("invalid"));
    }

    #[test]
    fn display_names_every_guest_error() {
        assert_eq!(
            GuestError::InvalidFrame("f".into()).to_string(),
            "invalid frame: f"
        );
        assert_eq!(
            GuestError::UnknownTopic("t".into()).to_string(),
            "unknown topic: t"
        );
        assert_eq!(
            GuestError::UnsupportedQos("q".into()).to_string(),
            "unsupported qos: q"
        );
        assert_eq!(
            GuestError::UnsupportedProto("p".into()).to_string(),
            "unsupported proto: p"
        );
        assert_eq!(GuestError::QueueFull.to_string(), "guest outbox full");
        assert_eq!(GuestError::NotConnected.to_string(), "not connected");
    }

    #[test]
    fn from_proto_error_maps_every_arm() {
        assert!(matches!(
            GuestError::from(ProtoError::Unsupported("u".into())),
            GuestError::UnsupportedProto(_)
        ));
        assert!(matches!(
            GuestError::from(ProtoError::Serialization("s".into())),
            GuestError::InvalidFrame(_)
        ));
        let mapped = GuestError::from(ProtoError::TooLarge { got: 10, max: 8 }).to_string();
        assert!(mapped.contains("frame too large: 10 (max 8)"), "{mapped}");
        assert!(matches!(
            GuestError::from(ProtoError::InvalidFrame("i")),
            GuestError::InvalidFrame(_)
        ));
        assert!(matches!(
            GuestError::from(ProtoError::NotConnected),
            GuestError::NotConnected
        ));
    }

    #[test]
    fn handle_frame_rejects_non_cdr_flags() {
        let mut g = GuestDispatcher::new(&["T"], 8);
        let frame = DataFrame {
            proto: PROTO_VERSION,
            flags: 0,
            topic: "T".into(),
            cdr: vec![1, 2, 3],
            seq: 0,
        }
        .encode()
        .unwrap();
        let err = g.handle_frame(&frame).unwrap_err().to_string();
        assert!(err.contains("requires CDR-LE"), "{err}");
    }

    #[test]
    fn control_plane_unsubscribes_registers_and_ignores_bye_ack_error() {
        let mut g = GuestDispatcher::new(&["T"], 8);
        let unsub = Control::Unsubscribe {
            proto: PROTO_VERSION,
            topic: "T".into(),
            seq: 9,
        }
        .to_json()
        .unwrap();
        let reply = g.handle_control(&unsub).unwrap().expect("ack");
        assert!(matches!(
            Control::from_json(&reply).unwrap(),
            Control::Ack { seq: 9, .. }
        ));
        assert!(g.subscriptions().is_empty());

        let reg = Control::Register {
            proto: PROTO_VERSION,
            topic: "N".into(),
            type_name: "WasmEcho".into(),
            qos: Qos::default(),
            seq: 10,
        }
        .to_json()
        .unwrap();
        let reply = g.handle_control(&reg).unwrap().expect("ack");
        assert!(matches!(
            Control::from_json(&reply).unwrap(),
            Control::Ack { seq: 10, .. }
        ));
        assert_eq!(g.subscriptions(), vec!["N".to_string()]);

        for ignored in [
            Control::Bye {
                proto: PROTO_VERSION,
            }
            .to_json()
            .unwrap(),
            Control::Ack {
                proto: PROTO_VERSION,
                seq: 1,
            }
            .to_json()
            .unwrap(),
            Control::Error {
                proto: PROTO_VERSION,
                code: "c".into(),
                detail: "d".into(),
            }
            .to_json()
            .unwrap(),
        ] {
            assert!(g.handle_control(&ignored).unwrap().is_none());
        }
    }

    #[test]
    fn guest_has_no_native_backend_in_its_graph() {
        let out = std::process::Command::new("cargo")
            .args([
                "tree",
                "-p",
                "dds-wasm-guest",
                "--edges",
                "normal",
                "-i",
                "cyclonedds-rust-sys",
            ])
            .output();
        // Hermetic check: `cargo tree` always runs inside `cargo test`,
        // and the native backend must be absent from the guest graph.
        let o = out.expect("cargo tree runs under cargo test");
        let text =
            String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr);
        assert!(
            text.contains("nothing to print") || !text.contains("cyclonedds-rust-sys v"),
            "guest must not link sys: {text}"
        );
    }
}
