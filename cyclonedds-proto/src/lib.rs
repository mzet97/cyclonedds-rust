//! Portable bridge-protocol contracts (`proto v0`).
//!
//! Shared by every backend (native bridge, browser-gateway, WASI host,
//! future wasm engine) so the wire format has a single source of truth.
//! This crate is intentionally dependency-light and `no_std`-compatible:
//! it must never depend on `cyclonedds-rust-sys`, `wasm-bindgen` or
//! `web-sys`. See `docs/wasm/PROTOCOL.md` (proto v0 section).
//!
//! Two data planes exist:
//!
//! - **Legacy JSON** (`{"topic","data"}`): what `cyclonedds-wasm` speaks
//!   today. Kept behind an explicit flag after migration.
//! - **CDR binary frame**: `u16 proto | u16 flags | u32 topic_len |
//!   topic bytes | u32 cdr_len | CDR bytes | u32 seq` (all LE).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Wire protocol version implemented by this crate.
pub const PROTO_VERSION: u16 = 0;

/// Frame flag: payload is CDR little-endian.
pub const FLAG_CDR_LE: u16 = 0x01;
/// Frame flag: payload is legacy JSON (never combined with binary flags).
pub const FLAG_LEGACY_JSON: u16 = 0x02;

/// Hard cap for a decoded frame payload (topic + CDR), 8 MiB.
/// Oversize frames are rejected with [`ProtoError::TooLarge`]; the
/// connection itself is never torn down by a single bad frame.
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Typed protocol error. `Unsupported` is *treatment*, not implementation:
/// reception paths must map unknown kinds/QoS here instead of panicking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtoError {
    Unsupported(String),
    Serialization(String),
    TooLarge { got: usize, max: usize },
    InvalidFrame(&'static str),
    NotConnected,
}

impl core::fmt::Display for ProtoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProtoError::Unsupported(s) => write!(f, "unsupported: {s}"),
            ProtoError::Serialization(s) => write!(f, "serialization error: {s}"),
            ProtoError::TooLarge { got, max } => {
                write!(f, "frame too large: {got} bytes (max {max})")
            }
            ProtoError::InvalidFrame(s) => write!(f, "invalid frame: {s}"),
            ProtoError::NotConnected => write!(f, "not connected"),
        }
    }
}

impl core::error::Error for ProtoError {}

pub type ProtoResult<T> = Result<T, ProtoError>;

pub mod echo;

// ---------------------------------------------------------------------------
// QoS (REQ-QOS-01: only best-effort + volatile; anything else is Unsupported)
// ---------------------------------------------------------------------------

/// Reliability subset modelled by the gateway profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reliability {
    BestEffort,
    /// Parsed but never accepted: requesting it yields `Unsupported`.
    Reliable,
}

/// Durability subset modelled by the gateway profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Durability {
    Volatile,
    /// Parsed but never accepted: requesting it yields `Unsupported`.
    TransientLocal,
}

/// QoS pair exchanged in `register`. Unknown future variants fail closed
/// at parse time (serde deny) rather than being silently accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Qos {
    pub reliability: Reliability,
    pub durability: Durability,
}

impl Default for Qos {
    fn default() -> Self {
        Qos {
            reliability: Reliability::BestEffort,
            durability: Durability::Volatile,
        }
    }
}

/// Enforce the gateway QoS subset. Non-best-effort/volatile requests
/// return `Unsupported` (treatment proof, not an implementation).
pub fn check_qos(qos: &Qos) -> ProtoResult<()> {
    if qos.reliability != Reliability::BestEffort {
        return Err(ProtoError::Unsupported("reliability != best_effort".into()));
    }
    if qos.durability != Durability::Volatile {
        return Err(ProtoError::Unsupported("durability != volatile".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Control plane (JSON, versioned)
// ---------------------------------------------------------------------------

/// Control message kinds (`kind` field). Unknown kinds must surface as
/// `Unsupported`, never panic the dispatcher.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Control {
    Hello {
        proto: u16,
        client: String,
    },
    Register {
        proto: u16,
        topic: String,
        #[serde(rename = "type")]
        type_name: String,
        qos: Qos,
        seq: u32,
    },
    Subscribe {
        proto: u16,
        topic: String,
        seq: u32,
    },
    Unsubscribe {
        proto: u16,
        topic: String,
        seq: u32,
    },
    Bye {
        proto: u16,
    },
    Ack {
        proto: u16,
        seq: u32,
    },
    Error {
        proto: u16,
        code: String,
        detail: String,
    },
}

impl Control {
    /// Protocol version carried by this message.
    pub fn proto(&self) -> u16 {
        match self {
            Control::Hello { proto, .. }
            | Control::Register { proto, .. }
            | Control::Subscribe { proto, .. }
            | Control::Unsubscribe { proto, .. }
            | Control::Bye { proto }
            | Control::Ack { proto, .. }
            | Control::Error { proto, .. } => *proto,
        }
    }

    /// Reject messages from a newer protocol with `Unsupported`
    /// (client downgrades or aborts; never mis-parses).
    pub fn check_version(&self) -> ProtoResult<()> {
        let got = self.proto();
        if got > PROTO_VERSION {
            return Err(ProtoError::Unsupported(alloc::format!(
                "proto {got} > supported {}",
                PROTO_VERSION
            )));
        }
        Ok(())
    }

    pub fn to_json(&self) -> ProtoResult<String> {
        serde_json::to_string(self).map_err(|e| ProtoError::Serialization(e.to_string()))
    }

    pub fn from_json(s: &str) -> ProtoResult<Self> {
        serde_json::from_str(s).map_err(|e| ProtoError::Serialization(e.to_string()))
    }
}

/// Legacy JSON data envelope (`{"topic","data"}`). No version, no
/// sequence — superseded by [`Control`] + [`DataFrame`]; kept behind an
/// explicit flag for migration only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyEnvelope {
    pub topic: String,
    pub data: serde_json::Value,
}

impl LegacyEnvelope {
    pub fn encode(topic: &str, data: &serde_json::Value) -> ProtoResult<String> {
        let env = LegacyEnvelope {
            topic: topic.into(),
            data: data.clone(),
        };
        serde_json::to_string(&env).map_err(|e| ProtoError::Serialization(e.to_string()))
    }

    pub fn decode(s: &str) -> ProtoResult<Self> {
        serde_json::from_str(s).map_err(|e| ProtoError::Serialization(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// Binary data frame: u16 proto | u16 flags | u32 topic_len | topic |
// u32 cdr_len | cdr | u32 seq  (all little-endian)
// ---------------------------------------------------------------------------

/// Decoded binary data frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataFrame {
    pub proto: u16,
    pub flags: u16,
    pub topic: String,
    pub cdr: Vec<u8>,
    pub seq: u32,
}

impl DataFrame {
    pub fn encode(&self) -> ProtoResult<Vec<u8>> {
        if self.flags & FLAG_LEGACY_JSON != 0 && self.flags & FLAG_CDR_LE != 0 {
            return Err(ProtoError::InvalidFrame(
                "legacy-json and binary flags are mutually exclusive",
            ));
        }
        let topic = self.topic.as_bytes();
        let total = 2 + 2 + 4 + topic.len() + 4 + self.cdr.len() + 4;
        if total > MAX_FRAME_BYTES {
            return Err(ProtoError::TooLarge {
                got: total,
                max: MAX_FRAME_BYTES,
            });
        }
        let mut out = Vec::with_capacity(total);
        out.extend_from_slice(&self.proto.to_le_bytes());
        out.extend_from_slice(&self.flags.to_le_bytes());
        out.extend_from_slice(&(topic.len() as u32).to_le_bytes());
        out.extend_from_slice(topic);
        out.extend_from_slice(&(self.cdr.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.cdr);
        out.extend_from_slice(&self.seq.to_le_bytes());
        Ok(out)
    }

    /// Decode a frame. Truncated/adversarial input yields a typed error,
    /// never a panic (fuzz this with the same corpus discipline as
    /// `cdr_deserialize_corpus`).
    pub fn decode(buf: &[u8]) -> ProtoResult<Self> {
        fn take<'b>(cur: &mut &'b [u8], n: usize) -> ProtoResult<&'b [u8]> {
            if cur.len() < n {
                return Err(ProtoError::InvalidFrame("truncated frame"));
            }
            let (h, t) = cur.split_at(n);
            *cur = t;
            Ok(h)
        }
        let u16le = |b: &[u8]| u16::from_le_bytes([b[0], b[1]]);
        let u32le = |b: &[u8]| u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        let mut cur = buf;

        let proto = u16le(take(&mut cur, 2)?);
        let flags = u16le(take(&mut cur, 2)?);
        if flags & FLAG_LEGACY_JSON != 0 && flags & FLAG_CDR_LE != 0 {
            return Err(ProtoError::InvalidFrame(
                "legacy-json and binary flags are mutually exclusive",
            ));
        }
        let topic_len = u32le(take(&mut cur, 4)?) as usize;
        let cdr_len_off = 4usize; // u32 cdr_len + u32 seq trailers
        if topic_len > MAX_FRAME_BYTES || cur.len() < topic_len + cdr_len_off + 4 {
            // Cheap length check first: avoids slicing before validating.
            if topic_len > MAX_FRAME_BYTES {
                return Err(ProtoError::TooLarge {
                    got: topic_len,
                    max: MAX_FRAME_BYTES,
                });
            }
            return Err(ProtoError::InvalidFrame("truncated frame"));
        }
        let topic_bytes = take(&mut cur, topic_len)?;
        let topic = core::str::from_utf8(topic_bytes)
            .map_err(|_| ProtoError::InvalidFrame("topic is not utf-8"))?;
        let cdr_len = u32le(take(&mut cur, 4)?) as usize;
        if cdr_len > MAX_FRAME_BYTES || cur.len() != cdr_len + 4 {
            if cdr_len > MAX_FRAME_BYTES {
                return Err(ProtoError::TooLarge {
                    got: cdr_len,
                    max: MAX_FRAME_BYTES,
                });
            }
            return Err(ProtoError::InvalidFrame("trailing bytes mismatch"));
        }
        let cdr = take(&mut cur, cdr_len)?.to_vec();
        let seq = u32le(take(&mut cur, 4)?);
        if proto > PROTO_VERSION {
            return Err(ProtoError::Unsupported(alloc::format!(
                "proto {proto} > supported {PROTO_VERSION}"
            )));
        }
        Ok(DataFrame {
            proto,
            flags,
            topic: topic.into(),
            cdr,
            seq,
        })
    }
}
