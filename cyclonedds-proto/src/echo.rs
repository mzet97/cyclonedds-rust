//! Minimal pure-Rust XCDR1 (little-endian) codec for the Phase C demo type.
//!
//! The demo IDL is `struct WasmEcho { @key long id; string text;
//! sequence<long> values; }`. This codec is the single source of truth for
//! the binary data plane: the browser client (`cyclonedds-wasm`), the
//! native gateway (`dds-wasm-bridge`), and the wire-compat test against
//! `libddsc` (`CdrSerializer`/`CdrDeserializer`) all use these exact bytes.
//!
//! Layout (XCDR1 LE, matching `dds_stream_write_sample` for this type —
//! verified byte-for-byte in `dds-wasm-bridge` compat tests, no
//! encapsulation header, exactly what `CdrSerializer::serialize` emits):
//!
//! ```text
//! i32 id | u32 text_len (= utf8_len + 1) | utf8 bytes | 0x00 | pad-to-4
//! | u32 seq_len | seq_len × i32 LE
//! ```
//!
//! Decoding never panics: truncated, over-long, or non-UTF8 input yields a
//! typed [`ProtoError`], and declared lengths are capped before any
//! allocation.

use crate::{ProtoError, ProtoResult};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Topic name shared by every Phase C participant.
pub const ECHO_TOPIC: &str = "WasmEcho";
/// Type name advertised in `register`.
pub const ECHO_TYPE: &str = "WasmEcho";

/// Cap for `text` UTF-8 bytes (excluding NUL). Larger declarations are
/// rejected before allocating.
pub const MAX_ECHO_TEXT: usize = 1024 * 1024;
/// Cap for `values` element count. Larger declarations are rejected before
/// allocating.
pub const MAX_ECHO_VALUES: usize = 262_144; // 1 MiB of i32

/// Portable demo sample: key + string + sequence.
///
/// Serde applies to the legacy-JSON compat path only; the binary data
/// plane always uses [`encode_echo_xcdr1`]/[`decode_echo_xcdr1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EchoMsg {
    pub id: i32,
    pub text: String,
    pub values: Vec<i32>,
}

fn read_u32le(cur: &mut &[u8]) -> ProtoResult<u32> {
    if cur.len() < 4 {
        return Err(ProtoError::InvalidFrame("truncated echo cdr"));
    }
    let (h, t) = cur.split_at(4);
    *cur = t;
    Ok(u32::from_le_bytes([h[0], h[1], h[2], h[3]]))
}

fn read_bytes<'b>(cur: &mut &'b [u8], n: usize) -> ProtoResult<&'b [u8]> {
    if cur.len() < n {
        return Err(ProtoError::InvalidFrame("truncated echo cdr"));
    }
    let (h, t) = cur.split_at(n);
    *cur = t;
    Ok(h)
}

/// Encode `msg` to XCDR1 LE bytes (no encapsulation header).
pub fn encode_echo_xcdr1(msg: &EchoMsg) -> ProtoResult<Vec<u8>> {
    let text = msg.text.as_bytes();
    if text.len() > MAX_ECHO_TEXT {
        return Err(ProtoError::TooLarge {
            got: text.len(),
            max: MAX_ECHO_TEXT,
        });
    }
    if msg.values.len() > MAX_ECHO_VALUES {
        return Err(ProtoError::TooLarge {
            got: msg.values.len() * 4,
            max: MAX_ECHO_VALUES * 4,
        });
    }
    if text.contains(&0) {
        return Err(ProtoError::Serialization(
            "echo text contains interior NUL".into(),
        ));
    }
    let text_field = text.len() + 1; // + NUL
    let pad = (4 - (text_field % 4)) % 4;
    let mut out = Vec::with_capacity(4 + 4 + text_field + pad + 4 + msg.values.len() * 4);
    out.extend_from_slice(&msg.id.to_le_bytes());
    out.extend_from_slice(&(text_field as u32).to_le_bytes());
    out.extend_from_slice(text);
    out.push(0);
    out.extend(core::iter::repeat_n(0, pad));
    out.extend_from_slice(&(msg.values.len() as u32).to_le_bytes());
    for v in &msg.values {
        out.extend_from_slice(&v.to_le_bytes());
    }
    Ok(out)
}

/// Decode XCDR1 LE bytes into an [`EchoMsg`]. Never panics; malformed or
/// adversarial input yields [`ProtoError::InvalidFrame`],
/// [`ProtoError::Serialization`] (bad UTF-8 / missing NUL), or
/// [`ProtoError::TooLarge`]. Trailing bytes are rejected.
pub fn decode_echo_xcdr1(buf: &[u8]) -> ProtoResult<EchoMsg> {
    let mut cur = buf;
    let id = read_u32le(&mut cur)? as i32;
    let text_len = read_u32le(&mut cur)? as usize;
    if text_len == 0 || text_len - 1 > MAX_ECHO_TEXT {
        if text_len == 0 {
            return Err(ProtoError::InvalidFrame("echo string has zero length"));
        }
        return Err(ProtoError::TooLarge {
            got: text_len - 1,
            max: MAX_ECHO_TEXT,
        });
    }
    let raw = read_bytes(&mut cur, text_len)?;
    if raw[text_len - 1] != 0 {
        return Err(ProtoError::Serialization(
            "echo string is not NUL-terminated".into(),
        ));
    }
    let text = core::str::from_utf8(&raw[..text_len - 1])
        .map_err(|_| ProtoError::Serialization("echo text is not utf-8".into()))?;
    let pad = (4 - (text_len % 4)) % 4;
    read_bytes(&mut cur, pad)?;
    let seq_len = read_u32le(&mut cur)? as usize;
    if seq_len > MAX_ECHO_VALUES {
        return Err(ProtoError::TooLarge {
            got: seq_len * 4,
            max: MAX_ECHO_VALUES * 4,
        });
    }
    // `seq_len <= MAX_ECHO_VALUES` (checked above): this reserves at most
    // 1 MiB, so the reservation cannot fail on any supported target and
    // needs no fallible path of its own.
    let mut values = Vec::with_capacity(seq_len);
    if seq_len > 0 {
        let words = read_bytes(&mut cur, seq_len * 4)?;
        for chunk in words.chunks_exact(4) {
            values.push(i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
    }
    if !cur.is_empty() {
        return Err(ProtoError::InvalidFrame("trailing bytes in echo cdr"));
    }
    Ok(EchoMsg {
        id,
        text: String::from(text),
        values,
    })
}
