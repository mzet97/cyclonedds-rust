//! Experimental DDS bindings for WebAssembly using WebSocket transport.
//!
//! This crate provides a DDS-compatible API that runs in the browser via
//! WebAssembly. It uses WebSocket as the underlying transport instead of
//! the native CycloneDDS C library (which cannot compile to WASM).
//!
//! # Limitations
//!
//! - This is **not** a full DDS implementation. It uses JSON over WebSocket
//!   rather than the OMG DDS wire protocol (RTPS).
//! - A DDS-to-WebSocket bridge server is required to communicate with
//!   native DDS participants.
//! - Only best-effort, volatile durability is supported.
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

use serde::{de::DeserializeOwned, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};

/// Error type for WASM DDS operations.
#[derive(Debug, Clone)]
pub enum WasmDdsError {
    WebSocket(String),
    Serialization(String),
    NotConnected,
    TopicNotFound(String),
}

impl std::fmt::Display for WasmDdsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WasmDdsError::WebSocket(s) => write!(f, "WebSocket error: {s}"),
            WasmDdsError::Serialization(s) => write!(f, "serialization error: {s}"),
            WasmDdsError::NotConnected => write!(f, "not connected"),
            WasmDdsError::TopicNotFound(s) => write!(f, "topic not found: {s}"),
        }
    }
}

impl std::error::Error for WasmDdsError {}

pub type WasmDdsResult<T> = Result<T, WasmDdsError>;

/// A DDS domain participant backed by a WebSocket connection.
pub struct WasmDomainParticipant {
    ws: WebSocket,
    topics: RefCell<HashMap<String, String>>, // name -> type_name
}

impl WasmDomainParticipant {
    /// Connect to a DDS WebSocket bridge.
    pub fn new(url: &str) -> WasmDdsResult<Rc<Self>> {
        let ws = WebSocket::new(url).map_err(|e| {
            WasmDdsError::WebSocket(format!("{:?}", e))
        })?;
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        let participant = Rc::new(WasmDomainParticipant {
            ws,
            topics: RefCell::new(HashMap::new()),
        });

        let onopen = Closure::wrap(Box::new(move || {
            web_sys::console::log_1(&"DDS WebSocket connected".into());
        }) as Box<dyn Fn()>);
        participant.ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();

        let onerror = Closure::wrap(Box::new(move |e: ErrorEvent| {
            web_sys::console::error_1(&format!("DDS WebSocket error: {:?}", e).into());
        }) as Box<dyn Fn(ErrorEvent)>);
        participant.ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
        onerror.forget();

        Ok(participant)
    }

    /// Create a topic.
    pub fn create_topic<T: Serialize + DeserializeOwned>(
        self: &Rc<Self>,
        name: &str,
    ) -> WasmDdsResult<WasmTopic<T>> {
        let type_name = std::any::type_name::<T>();
        self.topics.borrow_mut().insert(name.to_string(), type_name.to_string());
        Ok(WasmTopic {
            name: name.to_string(),
            _marker: std::marker::PhantomData,
        })
    }

    /// Create a writer for a topic.
    pub fn create_writer<T: Serialize>(
        self: &Rc<Self>,
        topic: &WasmTopic<T>,
    ) -> WasmDdsResult<WasmDataWriter<T>> {
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
        let topic_name = topic.name.clone();
        let ws = self.ws.clone();

        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                let txt = String::from(txt);
                if let Ok(mut envelope) = serde_json::from_str::<serde_json::Value>(&txt) {
                    if let Some(t) = envelope.get("topic") {
                        if t.as_str() == Some(&topic_name) {
                            if let Some(payload) = envelope.get_mut("data") {
                                if let Ok(data) = serde_json::from_value::<T>(payload.take()) {
                                    on_data(data);
                                }
                            }
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
    /// Publish a sample.
    pub fn write(&self, data: &T) -> WasmDdsResult<()> {
        let payload = serde_json::to_value(data)
            .map_err(|e| WasmDdsError::Serialization(e.to_string()))?;
        let envelope = serde_json::json!({
            "topic": self.topic_name,
            "data": payload,
        });
        let json = serde_json::to_string(&envelope)
            .map_err(|e| WasmDdsError::Serialization(e.to_string()))?;
        self.ws.send_with_str(&json).map_err(|e| {
            WasmDdsError::WebSocket(format!("{:?}", e))
        })?;
        Ok(())
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
