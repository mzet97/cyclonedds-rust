//! Experimental DDS bindings for WebAssembly using WebSocket transport.
//!
//! This crate provides a DDS-compatible API that runs in the browser via
//! WebAssembly. It uses WebSocket as the underlying transport instead of
//! the native CycloneDDS C library (which cannot compile to WASM).
//!
//! Backend rule: this crate never depends on the `native` backend
//! (`cyclonedds-rust-sys` / libddsc). Portable wire contracts live in
//! [`proto`] (`cyclonedds-proto`, proto v0). The generic transport still
//! speaks the legacy JSON envelope (`{"topic","data"}`); the binary CDR
//! data plane is wired for the Phase C echo type
//! ([`encode_echo_frame`]/[`decode_echo_frame`], `WasmEchoWriter`, pure and
//! host-testable, `ArrayBuffer` send on wasm32). The versioned control
//! plane (`hello`/`ack`) is NOT yet wired here (gap preserved, see
//! `docs/wasm/PROTOCOL.md` and `docs/wasm/PHASE_C.md`).
//!
//! # Limitations
//!
//! - This is **not** a full DDS implementation. It uses JSON over WebSocket
//!   rather than the OMG DDS wire protocol (RTPS).
//! - A DDS-to-WebSocket bridge server is required to communicate with
//!   native DDS participants.
//! - Only best-effort, volatile durability is supported: requesting any
//!   other QoS returns [`WasmDdsError::Unsupported`] (treatment, not an
//!   implementation of reliable/durable delivery).
//! - The live `WebSocket` transport exists only on `wasm32` targets. On a
//!   host target the same API links but I/O methods return `NotConnected`,
//!   so portable logic (envelope encode, QoS checks) stays testable
//!   without a browser.
//!
//! # Example
//!
//! ```ignore
//! use cyclonedds_wasm::*;
//! use wasm_bindgen::prelude::*;
//!
//! #[derive(Serialize, Deserialize)]
//! struct MyMessage { id: i32, text: String }
//!
//! #[wasm_bindgen(start)]
//! pub fn main() {
//!     let participant = WasmDomainParticipant::new("ws://localhost:8080/dds")
//!         .expect("connect");
//!     let topic = participant.create_topic::<MyMessage>("HelloWorld")
//!         .expect("create topic");
//!     let writer = participant.create_writer(&topic)
//!         .expect("create writer");
//!
//!     let msg = MyMessage { id: 1, text: "hello".to_string() };
//!     writer.write(&msg).expect("write");
//! }
//! ```

pub use cyclonedds_proto as proto;

pub mod bigint;
pub use bigint::{
    decimal_str_to_i64, decimal_str_to_u64, i64_to_decimal_str, u64_needs_bigint,
    u64_to_decimal_str, MAX_SAFE_INTEGER_U64,
};

#[cfg(target_arch = "wasm32")]
pub mod js;

use cyclonedds_proto::echo::{decode_echo_xcdr1, encode_echo_xcdr1};
use cyclonedds_proto::{
    check_qos, DataFrame, LegacyEnvelope, ProtoError, Qos, FLAG_CDR_LE, PROTO_VERSION,
};
use serde::{de::DeserializeOwned, Serialize};

/// Error type for WASM DDS operations.
#[derive(Debug, Clone)]
pub enum WasmDdsError {
    WebSocket(String),
    Serialization(String),
    NotConnected,
    TopicNotFound(String),
    /// An observable wait (open handshake, sample arrival) exceeded its
    /// deadline. Timeouts are first-class: callers can distinguish "no data
    /// yet" from transport failure.
    Timeout,
    /// Requested feature/QoS is not implemented by this profile.
    /// Returning `Unsupported` proves *treatment*, not implementation.
    Unsupported(String),
}

impl std::fmt::Display for WasmDdsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmDdsError::WebSocket(s) => write!(f, "WebSocket error: {s}"),
            WasmDdsError::Serialization(s) => write!(f, "serialization error: {s}"),
            WasmDdsError::NotConnected => write!(f, "not connected"),
            WasmDdsError::TopicNotFound(s) => write!(f, "topic not found: {s}"),
            WasmDdsError::Timeout => write!(f, "timed out"),
            WasmDdsError::Unsupported(s) => write!(f, "unsupported: {s}"),
        }
    }
}

impl std::error::Error for WasmDdsError {}

impl From<ProtoError> for WasmDdsError {
    fn from(e: ProtoError) -> Self {
        match e {
            ProtoError::Unsupported(s) => WasmDdsError::Unsupported(s),
            ProtoError::Serialization(s) => WasmDdsError::Serialization(s),
            ProtoError::TooLarge { got, max } => {
                WasmDdsError::Serialization(format!("frame too large: {got} (max {max})"))
            }
            ProtoError::InvalidFrame(s) => {
                WasmDdsError::Serialization(format!("invalid frame: {s}"))
            }
            ProtoError::NotConnected => WasmDdsError::NotConnected,
        }
    }
}

pub type WasmDdsResult<T> = Result<T, WasmDdsError>;

/// Encode a sample into the legacy JSON envelope via the portable
/// [`proto`] contract, so every backend produces byte-identical envelopes.
/// Pure function: runs on any target, no socket needed.
pub fn encode_sample<T: Serialize>(topic: &str, data: &T) -> WasmDdsResult<String> {
    let payload =
        serde_json::to_value(data).map_err(|e| WasmDdsError::Serialization(e.to_string()))?;
    Ok(LegacyEnvelope::encode(topic, &payload)?)
}

/// Validate gateway QoS before creating readers/writers.
/// Anything other than best-effort + volatile yields `Unsupported`.
pub fn check_gateway_qos(qos: &Qos) -> WasmDdsResult<()> {
    Ok(check_qos(qos)?)
}

// ---------------------------------------------------------------------------
// Binary data plane (Phase C): CDR frames, pure functions, any target.
// ---------------------------------------------------------------------------

/// Encode one `EchoMsg` sample into a binary [`DataFrame`] (`proto v0`,
/// `FLAG_CDR_LE`). This is the browser data-plane send path; the legacy
/// JSON envelope ([`encode_sample`]) stays available only as explicit
/// compat and must never be produced here.
pub fn encode_echo_frame(topic: &str, msg: &EchoMsg, seq: u32) -> WasmDdsResult<Vec<u8>> {
    let cdr = encode_echo_xcdr1(msg)?;
    let frame = DataFrame {
        proto: PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: topic.into(),
        cdr,
        seq,
    };
    Ok(frame.encode()?)
}

/// Decode one binary frame into `(topic, sample, seq)`. Rejects legacy-JSON
/// frames and newer protos with typed errors (never panics, never
/// mis-parses).
pub fn decode_echo_frame(buf: &[u8]) -> WasmDdsResult<(String, EchoMsg, u32)> {
    let frame = DataFrame::decode(buf)?;
    if frame.flags != FLAG_CDR_LE {
        return Err(WasmDdsError::Unsupported(format!(
            "echo data plane requires CDR-LE frames, got flags {:#06x} \
             (legacy JSON is compat-only, behind an explicit flag)",
            frame.flags
        )));
    }
    let msg = decode_echo_xcdr1(&frame.cdr)?;
    Ok((frame.topic, msg, frame.seq))
}

// ---------------------------------------------------------------------------
// wasm32 transport (live WebSocket)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod backend {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use wasm_bindgen::prelude::*;
    use web_sys::{ErrorEvent, MessageEvent, WebSocket};

    /// A DDS domain participant backed by a WebSocket connection.
    pub struct WasmDomainParticipant {
        ws: WebSocket,
        topics: RefCell<HashMap<String, String>>, // name -> type_name
    }

    impl WasmDomainParticipant {
        /// Connect to a DDS WebSocket bridge.
        pub fn new(url: &str) -> WasmDdsResult<Rc<Self>> {
            let ws =
                WebSocket::new(url).map_err(|e| WasmDdsError::WebSocket(format!("{:?}", e)))?;
            ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

            let participant = Rc::new(WasmDomainParticipant {
                ws,
                topics: RefCell::new(HashMap::new()),
            });

            let onopen = Closure::wrap(Box::new(move || {
                web_sys::console::log_1(&"DDS WebSocket connected".into());
            }) as Box<dyn Fn()>);
            participant
                .ws
                .set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();

            let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
                web_sys::console::error_1(&format!("DDS WebSocket error: {:?}", e).into());
            }) as Box<dyn Fn(ErrorEvent)>);
            participant
                .ws
                .set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();

            Ok(participant)
        }

        /// Create a topic.
        pub fn create_topic<T: Serialize + DeserializeOwned>(
            self: &Rc<Self>,
            name: &str,
        ) -> WasmDdsResult<WasmTopic<T>> {
            let type_name = std::any::type_name::<T>();
            self.topics
                .borrow_mut()
                .insert(name.to_string(), type_name.to_string());
            Ok(WasmTopic {
                name: name.to_string(),
                _marker: std::marker::PhantomData,
            })
        }

        /// Create a writer for a topic (default gateway QoS).
        pub fn create_writer<T: Serialize>(
            self: &Rc<Self>,
            topic: &WasmTopic<T>,
        ) -> WasmDdsResult<WasmDataWriter<T>> {
            self.create_writer_with_qos(topic, &Qos::default())
        }

        /// Create a writer, enforcing gateway QoS (REQ-QOS-01).
        pub fn create_writer_with_qos<T: Serialize>(
            self: &Rc<Self>,
            topic: &WasmTopic<T>,
            qos: &Qos,
        ) -> WasmDdsResult<WasmDataWriter<T>> {
            check_gateway_qos(qos)?;
            Ok(WasmDataWriter {
                ws: self.ws.clone(),
                topic_name: topic.name.clone(),
                _marker: std::marker::PhantomData,
            })
        }

        /// Create a reader for a topic with a callback.
        pub fn create_reader<T: DeserializeOwned + 'static>(
            self: &Rc<Self>,
            topic: &WasmTopic<T>,
            on_data: Box<dyn Fn(T)>,
        ) -> WasmDdsResult<WasmDataReader<T>> {
            self.create_reader_with_qos(topic, &Qos::default(), on_data)
        }

        /// Create a reader, enforcing gateway QoS (REQ-QOS-01).
        pub fn create_reader_with_qos<T: DeserializeOwned + 'static>(
            self: &Rc<Self>,
            topic: &WasmTopic<T>,
            qos: &Qos,
            on_data: Box<dyn Fn(T)>,
        ) -> WasmDdsResult<WasmDataReader<T>> {
            check_gateway_qos(qos)?;
            let topic_name = topic.name.clone();
            let ws = self.ws.clone();

            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                    let txt = String::from(txt);
                    // Accept the legacy envelope; ignore anything else
                    // (unknown-topic samples are dropped, never crash).
                    if let Ok(envelope) = LegacyEnvelope::decode(&txt) {
                        if envelope.topic == topic_name {
                            if let Ok(data) = serde_json::from_value::<T>(envelope.data.clone()) {
                                on_data(data);
                            }
                        }
                    }
                }
            }) as Box<dyn Fn(MessageEvent)>);
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            Ok(WasmDataReader {
                ws,
                topic_name: topic.name.clone(),
                _marker: std::marker::PhantomData,
            })
        }

        /// Create a binary (CDR) echo writer for a topic (default gateway QoS).
        pub fn create_echo_writer(
            self: &Rc<Self>,
            topic: &WasmTopic<EchoMsg>,
        ) -> WasmDdsResult<WasmEchoWriter> {
            check_gateway_qos(&Qos::default())?;
            Ok(WasmEchoWriter {
                ws: self.ws.clone(),
                topic_name: topic.name.clone(),
                seq: std::cell::Cell::new(0),
            })
        }

        /// Create a binary (CDR) echo reader for a topic.
        pub fn create_echo_reader(
            self: &Rc<Self>,
            topic: &WasmTopic<EchoMsg>,
            on_data: Box<dyn Fn(EchoMsg)>,
        ) -> WasmDdsResult<WasmDataReader<EchoMsg>> {
            check_gateway_qos(&Qos::default())?;
            let topic_name = topic.name.clone();
            let ws = self.ws.clone();

            let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
                // Binary data plane: ArrayBuffer carrying one DataFrame.
                if let Ok(buf) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                    let bytes = js_sys::Uint8Array::new(&buf).to_vec();
                    if let Ok((t, msg, _)) = super::decode_echo_frame(&bytes) {
                        if t == topic_name {
                            on_data(msg);
                        }
                    }
                }
            }) as Box<dyn Fn(MessageEvent)>);
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            Ok(WasmDataReader {
                ws,
                topic_name: topic.name.clone(),
                _marker: std::marker::PhantomData,
            })
        }

        /// Disconnect from the bridge.
        pub fn disconnect(&self) {
            let _ = self.ws.close();
        }
    }

    /// A topic handle in the WASM DDS runtime.
    pub struct WasmTopic<T> {
        name: String,
        _marker: std::marker::PhantomData<T>,
    }

    /// A writer that publishes JSON-serialized samples over WebSocket.
    pub struct WasmDataWriter<T> {
        ws: WebSocket,
        topic_name: String,
        _marker: std::marker::PhantomData<T>,
    }

    impl<T: Serialize> WasmDataWriter<T> {
        /// Publish a sample (legacy JSON envelope — explicit compat only).
        pub fn write(&self, data: &T) -> WasmDdsResult<()> {
            let json = super::encode_sample(&self.topic_name, data)?;
            self.ws
                .send_with_str(&json)
                .map_err(|e| WasmDdsError::WebSocket(format!("{:?}", e)))?;
            Ok(())
        }
    }

    /// Binary (CDR) echo writer: publishes [`EchoMsg`] samples as
    /// `ArrayBuffer` frames on the binary data plane.
    pub struct WasmEchoWriter {
        ws: WebSocket,
        topic_name: String,
        seq: std::cell::Cell<u32>,
    }

    impl WasmEchoWriter {
        /// Publish one sample as a binary CDR frame.
        pub fn write_echo(&self, msg: &EchoMsg) -> WasmDdsResult<()> {
            let seq = self.seq.get();
            self.seq.set(seq.wrapping_add(1));
            let frame = super::encode_echo_frame(&self.topic_name, msg, seq)?;
            self.ws
                .send_with_u8_array(&frame)
                .map_err(|e| WasmDdsError::WebSocket(format!("{:?}", e)))?;
            Ok(())
        }

        /// Topic name.
        pub fn topic_name(&self) -> &str {
            &self.topic_name
        }
    }

    /// A reader that receives JSON-deserialized samples over WebSocket.
    pub struct WasmDataReader<T> {
        #[allow(dead_code)]
        ws: WebSocket,
        topic_name: String,
        _marker: std::marker::PhantomData<T>,
    }

    impl<T> WasmDataReader<T> {
        /// Topic name.
        pub fn topic_name(&self) -> &str {
            &self.topic_name
        }
    }
}

// ---------------------------------------------------------------------------
// Host fallback: same API links without a browser; I/O returns NotConnected.
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod backend {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    /// Host-side stand-in: records topics locally, performs envelope/QoS
    /// logic portably, but has no socket, so I/O is `NotConnected`.
    /// (The browser transport gap is explicit, not discretized: this does
    /// not "implement" the gateway on host.)
    pub struct WasmDomainParticipant {
        url: String,
        topics: RefCell<HashMap<String, String>>,
    }

    impl WasmDomainParticipant {
        pub fn new(url: &str) -> WasmDdsResult<Rc<Self>> {
            Ok(Rc::new(WasmDomainParticipant {
                url: url.to_string(),
                topics: RefCell::new(HashMap::new()),
            }))
        }

        /// Bridge URL this participant was created with.
        pub fn url(&self) -> &str {
            &self.url
        }

        pub fn create_topic<T: Serialize + DeserializeOwned>(
            self: &Rc<Self>,
            name: &str,
        ) -> WasmDdsResult<WasmTopic<T>> {
            let type_name = std::any::type_name::<T>();
            self.topics
                .borrow_mut()
                .insert(name.to_string(), type_name.to_string());
            Ok(WasmTopic {
                name: name.to_string(),
                _marker: std::marker::PhantomData,
            })
        }

        pub fn create_writer<T: Serialize>(
            self: &Rc<Self>,
            topic: &WasmTopic<T>,
        ) -> WasmDdsResult<WasmDataWriter<T>> {
            self.create_writer_with_qos(topic, &Qos::default())
        }

        pub fn create_writer_with_qos<T: Serialize>(
            self: &Rc<Self>,
            topic: &WasmTopic<T>,
            qos: &Qos,
        ) -> WasmDdsResult<WasmDataWriter<T>> {
            check_gateway_qos(qos)?;
            Ok(WasmDataWriter {
                topic_name: topic.name.clone(),
                _marker: std::marker::PhantomData,
            })
        }

        pub fn create_reader<T: DeserializeOwned + 'static>(
            self: &Rc<Self>,
            topic: &WasmTopic<T>,
            _on_data: Box<dyn Fn(T)>,
        ) -> WasmDdsResult<WasmDataReader<T>> {
            self.create_reader_with_qos(topic, &Qos::default(), _on_data)
        }

        pub fn create_reader_with_qos<T: DeserializeOwned + 'static>(
            self: &Rc<Self>,
            topic: &WasmTopic<T>,
            qos: &Qos,
            _on_data: Box<dyn Fn(T)>,
        ) -> WasmDdsResult<WasmDataReader<T>> {
            check_gateway_qos(qos)?;
            Ok(WasmDataReader {
                topic_name: topic.name.clone(),
                _marker: std::marker::PhantomData,
            })
        }

        /// Host stand-in for the binary echo writer: validates framing, then
        /// reports the transport gap as `NotConnected`.
        pub fn create_echo_writer(
            self: &Rc<Self>,
            topic: &WasmTopic<EchoMsg>,
        ) -> WasmDdsResult<WasmEchoWriter> {
            check_gateway_qos(&Qos::default())?;
            Ok(WasmEchoWriter {
                topic_name: topic.name.clone(),
                seq: std::cell::Cell::new(0),
            })
        }

        /// Host stand-in for the binary echo reader: registers the
        /// callback signature, no socket behind it.
        pub fn create_echo_reader(
            self: &Rc<Self>,
            topic: &WasmTopic<EchoMsg>,
            _on_data: Box<dyn Fn(EchoMsg)>,
        ) -> WasmDdsResult<WasmDataReader<EchoMsg>> {
            check_gateway_qos(&Qos::default())?;
            Ok(WasmDataReader {
                topic_name: topic.name.clone(),
                _marker: std::marker::PhantomData,
            })
        }

        pub fn disconnect(&self) {}
    }

    pub struct WasmTopic<T> {
        name: String,
        _marker: std::marker::PhantomData<T>,
    }

    pub struct WasmDataWriter<T> {
        topic_name: String,
        _marker: std::marker::PhantomData<T>,
    }

    impl<T: Serialize> WasmDataWriter<T> {
        /// Host has no socket: envelope encoding is validated, then
        /// `NotConnected` is returned (typed treatment of the gap).
        pub fn write(&self, data: &T) -> WasmDdsResult<()> {
            let _ = super::encode_sample(&self.topic_name, data)?;
            Err(WasmDdsError::NotConnected)
        }
    }

    /// Host stand-in for the binary echo writer.
    pub struct WasmEchoWriter {
        topic_name: String,
        seq: std::cell::Cell<u32>,
    }

    impl WasmEchoWriter {
        /// Validates the CDR frame, then returns `NotConnected` (no socket).
        pub fn write_echo(&self, msg: &EchoMsg) -> WasmDdsResult<()> {
            let seq = self.seq.get();
            self.seq.set(seq.wrapping_add(1));
            let _ = super::encode_echo_frame(&self.topic_name, msg, seq)?;
            Err(WasmDdsError::NotConnected)
        }

        pub fn topic_name(&self) -> &str {
            &self.topic_name
        }
    }

    pub struct WasmDataReader<T> {
        topic_name: String,
        _marker: std::marker::PhantomData<T>,
    }

    impl<T> WasmDataReader<T> {
        pub fn topic_name(&self) -> &str {
            &self.topic_name
        }
    }
}

pub use backend::{
    WasmDataReader, WasmDataWriter, WasmDomainParticipant, WasmEchoWriter, WasmTopic,
};
pub use cyclonedds_proto::echo::EchoMsg;

#[cfg(test)]
mod portable_tests {
    use super::proto::{Durability, Qos, Reliability};
    use super::*;

    #[test]
    fn envelope_is_portable_and_wire_compatible() {
        #[derive(Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Msg {
            id: i32,
            text: String,
        }
        let json = encode_sample(
            "HelloWorld",
            &Msg {
                id: 1,
                text: "hello".into(),
            },
        )
        .unwrap();
        // Wire-identical to the historical hand-rolled envelope.
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["topic"], "HelloWorld");
        assert_eq!(v["data"], serde_json::json!({"id": 1, "text": "hello"}));
        let env = LegacyEnvelope::decode(&json).unwrap();
        assert_eq!(env.topic, "HelloWorld");
    }

    #[test]
    fn reliable_qos_is_unsupported_treatment() {
        let participant = WasmDomainParticipant::new("ws://localhost:9/dds").unwrap();
        let topic = participant.create_topic::<u32>("T").unwrap();
        let qos = Qos {
            reliability: Reliability::Reliable,
            durability: Durability::Volatile,
        };
        assert!(matches!(
            participant.create_writer_with_qos(&topic, &qos),
            Err(WasmDdsError::Unsupported(_))
        ));
        assert!(matches!(
            check_gateway_qos(&qos),
            Err(WasmDdsError::Unsupported(_))
        ));
    }

    #[test]
    fn echo_frame_roundtrips_with_cdr_flags() {
        use super::proto::echo::{EchoMsg, ECHO_TOPIC};
        use super::proto::FLAG_CDR_LE;
        let msg = EchoMsg {
            id: 42,
            text: "chave+string+sequência ✓".into(),
            values: vec![-1, 0, 7],
        };
        let bytes = super::encode_echo_frame(ECHO_TOPIC, &msg, 9).unwrap();
        let frame = super::proto::DataFrame::decode(&bytes).unwrap();
        assert_eq!(frame.flags, FLAG_CDR_LE);
        assert_eq!(frame.seq, 9);
        let (topic, back, seq) = super::decode_echo_frame(&bytes).unwrap();
        assert_eq!(topic, ECHO_TOPIC);
        assert_eq!(seq, 9);
        assert_eq!(back, msg);
    }

    #[test]
    fn echo_decode_rejects_legacy_flags_and_bad_cdr() {
        use super::proto::echo::EchoMsg;
        use super::proto::{DataFrame, FLAG_LEGACY_JSON, PROTO_VERSION};
        // Legacy-flagged frame carrying JSON-looking bytes: compat-only.
        let legacy = DataFrame {
            proto: PROTO_VERSION,
            flags: FLAG_LEGACY_JSON,
            topic: "WasmEcho".into(),
            cdr: br#"{"topic":"WasmEcho"}"#.to_vec(),
            seq: 0,
        }
        .encode()
        .unwrap();
        assert!(matches!(
            super::decode_echo_frame(&legacy),
            Err(WasmDdsError::Unsupported(_))
        ));
        // CDR-flagged frame with garbage CDR: typed serialization error.
        let bad_cdr = DataFrame {
            proto: PROTO_VERSION,
            flags: super::proto::FLAG_CDR_LE,
            topic: "WasmEcho".into(),
            cdr: vec![0xFF, 0x00],
            seq: 0,
        }
        .encode()
        .unwrap();
        assert!(matches!(
            super::decode_echo_frame(&bad_cdr),
            Err(WasmDdsError::Serialization(_))
        ));
        let _ = core::marker::PhantomData::<EchoMsg>;
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn host_echo_write_is_typed_not_connected() {
        use super::proto::echo::{EchoMsg, ECHO_TOPIC};
        let participant = WasmDomainParticipant::new("ws://localhost:9/dds").unwrap();
        let topic = participant.create_topic::<EchoMsg>(ECHO_TOPIC).unwrap();
        let writer = participant.create_echo_writer(&topic).unwrap();
        let msg = EchoMsg {
            id: 1,
            text: "x".into(),
            values: vec![],
        };
        assert!(matches!(
            writer.write_echo(&msg),
            Err(WasmDdsError::NotConnected)
        ));
        let _ = participant
            .create_echo_reader(&topic, Box::new(|_| {}))
            .unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn host_write_is_typed_not_connected() {
        let participant = WasmDomainParticipant::new("ws://localhost:9/dds").unwrap();
        let topic = participant.create_topic::<u32>("T").unwrap();
        let writer = participant.create_writer(&topic).unwrap();
        assert!(matches!(
            writer.write(&1u32),
            Err(WasmDdsError::NotConnected)
        ));
    }
}
