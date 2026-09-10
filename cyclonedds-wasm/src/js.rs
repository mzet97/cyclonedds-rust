//! wasm-bindgen export surface: the consumable JS/TS package (wasm32 only).
//!
//! This module is compiled ONLY for `wasm32` (see `Cargo.toml`: browser
//! deps are target-gated, and `cargo tree` audits must stay clean on
//! host). It exposes:
//!
//! - [`WasmClient::connect`] — async initialization returning a JS
//!   `Promise<WasmClient>` that resolves after the WebSocket `open`
//!   handshake (never reports success early) and rejects with a typed
//!   string on error, close-before-open, or timeout;
//! - `send_echo` / `on_echo` — binary CDR data plane (`proto v0`,
//!   `FLAG_CDR_LE`) over `ArrayBuffer`; legacy JSON is never produced;
//! - `u64_to_bigint` / `bigint_to_u64` / `i64_to_bigint` /
//!   `bigint_to_i64` — 64-bit integers cross as JS `bigint`, never as
//!   `Number` (exact above 2^53; see [`crate::bigint`]).
//!
//! Handler cleanup: `connect` clears its one-shot open/error/close
//! handlers once settled; [`WasmClient::close`] is idempotent and clears
//! `onmessage`. Per-reader fan-out (REQ-MUX-01) remains a registered gap.

use crate::bigint::{decimal_str_to_i64, decimal_str_to_u64};
use crate::{decode_echo_frame, encode_echo_frame};
use js_sys::{ArrayBuffer, BigInt, Function, Int32Array, Object, Promise, Reflect, Uint8Array};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{BinaryType, CloseEvent, ErrorEvent, MessageEvent, WebSocket};

/// Typed rejection reasons (all plain strings on the JS side).
const ERR_TIMEOUT: &str = "timeout: bridge WebSocket did not open in time";
const ERR_CLOSED: &str = "connection closed before open";

fn js_err(msg: impl Into<String>) -> JsValue {
    JsValue::from_str(&msg.into())
}

fn schedule_timeout(
    ms: u32,
    fire: Rc<Cell<bool>>,
    ws: WebSocket,
    reject: Function,
) -> Option<JsValue> {
    if ms == 0 {
        return None;
    }
    let global = js_sys::global();
    let set_timeout = Reflect::get(&global, &JsValue::from_str("setTimeout")).ok()?;
    let set_timeout = Function::from(set_timeout);
    let cb = Closure::once(move || {
        if fire.get() {
            fire.set(false);
            let _ = ws.close();
            let _ = reject.call1(&JsValue::NULL, &js_err(ERR_TIMEOUT));
        }
    });
    let id = set_timeout
        .call2(
            &JsValue::NULL,
            cb.as_ref().unchecked_ref(),
            &JsValue::from(ms),
        )
        .ok()?;
    cb.forget(); // one-shot; runs at most once.
    Some(id)
}

fn clear_timeout(id: &JsValue) {
    let global = js_sys::global();
    if let Ok(clear) = Reflect::get(&global, &JsValue::from_str("clearTimeout")) {
        let clear = Function::from(clear);
        let _ = clear.call1(&JsValue::NULL, id);
    }
}

/// Decode one binary frame into a JS object
/// `{topic, id, text, values: Int32Array, seq}`.
fn echo_frame_to_js(topic: String, msg: &crate::EchoMsg, seq: u32) -> Result<JsValue, JsValue> {
    let obj = Object::new();
    Reflect::set(
        &obj,
        &JsValue::from_str("topic"),
        &JsValue::from_str(&topic),
    )
    .map_err(|_| js_err("failed to build echo object"))?;
    Reflect::set(&obj, &JsValue::from_str("id"), &JsValue::from(msg.id))
        .map_err(|_| js_err("failed to build echo object"))?;
    Reflect::set(
        &obj,
        &JsValue::from_str("text"),
        &JsValue::from_str(&msg.text),
    )
    .map_err(|_| js_err("failed to build echo object"))?;
    Reflect::set(
        &obj,
        &JsValue::from_str("values"),
        &Int32Array::from(msg.values.as_slice()),
    )
    .map_err(|_| js_err("failed to build echo object"))?;
    Reflect::set(&obj, &JsValue::from_str("seq"), &JsValue::from(seq))
        .map_err(|_| js_err("failed to build echo object"))?;
    Ok(obj.into())
}

/// A connected bridge client. Construct only via [`WasmClient::connect`].
#[wasm_bindgen]
pub struct WasmClient {
    ws: WebSocket,
    seq: Cell<u32>,
    onmessage_closure: RefCell<Option<Closure<dyn FnMut(MessageEvent)>>>,
}

#[wasm_bindgen]
impl WasmClient {
    /// Open the bridge connection. The promise resolves with the client
    /// after `open`, or rejects with `"timeout: ..."` /
    /// `"connection closed before open"` / the socket error string.
    /// `timeout_ms == 0` disables the client-side timeout (callers should
    /// then `Promise.race` their own deadline, see `js/example.mjs`).
    pub fn connect(url: String, timeout_ms: u32) -> Promise {
        // Socket creation errors are synchronous: fail the same way async
        // callers observe (a rejected promise, not a throw).
        let ws = match WebSocket::new(&url) {
            Ok(ws) => ws,
            Err(e) => {
                return Promise::reject(&js_err(format!("WebSocket error: {e:?}")));
            }
        };
        ws.set_binary_type(BinaryType::Arraybuffer);

        let mut resolve_fn: Option<Function> = None;
        let mut reject_fn: Option<Function> = None;
        let promise = Promise::new(&mut |resolve, reject| {
            resolve_fn = Some(resolve);
            reject_fn = Some(reject);
        });
        let resolve = resolve_fn.expect("promise executor runs synchronously");
        let reject = reject_fn.expect("promise executor runs synchronously");

        // Shared settlement flag: exactly one of open/error/close/timeout
        // wins; losers are no-ops.
        let settled = Rc::new(Cell::new(false));
        let pending = Rc::new(Cell::new(true));

        // Timeout (best effort: skipped when the host has no setTimeout).
        let timer_id = schedule_timeout(timeout_ms, pending.clone(), ws.clone(), reject.clone());

        // open -> resolve(client)
        {
            let ws2 = ws.clone();
            let resolve = resolve.clone();
            let settled = settled.clone();
            let pending = pending.clone();
            let timer_id = timer_id.clone();
            // `once_into_js` (no `forget`): the one-shot allocation is
            // reclaimed after it fires instead of leaking per connect.
            let onopen: JsValue = Closure::once_into_js(move || {
                if !pending.get() || settled.get() {
                    return;
                }
                pending.set(false);
                settled.set(true);
                ws2.set_onopen(None);
                ws2.set_onerror(None);
                ws2.set_onclose(None);
                if let Some(id) = &timer_id {
                    clear_timeout(id);
                }
                let client = WasmClient {
                    ws: ws2,
                    seq: Cell::new(0),
                    onmessage_closure: RefCell::new(None),
                };
                let _ = resolve.call1(&JsValue::NULL, &JsValue::from(client));
            });
            ws.set_onopen(Some(onopen.unchecked_ref::<Function>()));
        }

        // error / close-before-open -> reject(typed string)
        {
            let fail = Rc::new({
                let ws = ws.clone();
                let reject = reject.clone();
                let settled = settled.clone();
                let pending = pending.clone();
                let timer_id = timer_id.clone();
                move |reason: String| {
                    if !pending.get() || settled.get() {
                        return;
                    }
                    pending.set(false);
                    settled.set(true);
                    ws.set_onopen(None);
                    ws.set_onerror(None);
                    ws.set_onclose(None);
                    if let Some(id) = &timer_id {
                        clear_timeout(id);
                    }
                    let _ = reject.call1(&JsValue::NULL, &js_err(reason));
                }
            });
            let fail_err = fail.clone();
            let onerror: JsValue = Closure::once_into_js(move |e: ErrorEvent| {
                fail_err(format!("WebSocket error: {e:?}"))
            });
            ws.set_onerror(Some(onerror.unchecked_ref::<Function>()));
            let onclose: JsValue =
                Closure::once_into_js(move |_: CloseEvent| fail(ERR_CLOSED.to_string()));
            ws.set_onclose(Some(onclose.unchecked_ref::<Function>()));
        }

        promise
    }

    /// Publish one echo sample as a binary CDR frame (`ArrayBuffer`).
    /// Throws a JS error string on serialization or socket failure.
    pub fn send_echo(
        &self,
        topic: &str,
        id: i32,
        text: &str,
        values: Vec<i32>,
    ) -> Result<(), JsValue> {
        let msg = crate::EchoMsg {
            id,
            text: text.to_string(),
            values,
        };
        let seq = self.seq.get();
        self.seq.set(seq.wrapping_add(1));
        let frame = encode_echo_frame(topic, &msg, seq)
            .map_err(|e| js_err(format!("serialization error: {e}")))?;
        self.ws
            .send_with_u8_array(&frame)
            .map_err(|e| js_err(format!("WebSocket error: {e:?}")))?;
        Ok(())
    }

    /// Register the echo callback (`(sample) => void`, sample shaped as
    /// `{topic, id, text, values: Int32Array, seq}`). Replaces any previous
    /// callback. Frames for other topics, non-CDR flags, or malformed CDR
    /// are dropped silently (decode-error counter gap, REQ-OBS-01).
    pub fn on_echo(&self, callback: Function) {
        let ws = self.ws.clone();
        let onmessage = Closure::wrap(Box::new(move |e: MessageEvent| {
            let buf: ArrayBuffer = match e.data().dyn_into() {
                Ok(b) => b,
                Err(_) => return, // text frames are compat-only: ignore.
            };
            let bytes = Uint8Array::new(&buf).to_vec();
            if let Ok((topic, msg, seq)) = decode_echo_frame(&bytes) {
                if let Ok(obj) = echo_frame_to_js(topic, &msg, seq) {
                    let _ = callback.call1(&JsValue::NULL, &obj);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        *self.onmessage_closure.borrow_mut() = Some(onmessage);
    }

    /// Idempotent disconnect: closes the socket and clears `onmessage`.
    /// Calling twice never throws.
    pub fn close(&self) {
        self.ws.set_onmessage(None);
        self.onmessage_closure.borrow_mut().take();
        let _ = self.ws.close();
    }
}

// ---------------------------------------------------------------------------
// 64-bit integers: BigInt in, BigInt out. Never Number.
// ---------------------------------------------------------------------------

/// Wrap a Rust `u64` as a JS `bigint` (exact for the full range,
/// including values above `Number.MAX_SAFE_INTEGER`).
#[wasm_bindgen]
pub fn u64_to_bigint(v: u64) -> BigInt {
    BigInt::from(v)
}

/// Convert a JS `bigint` to a Rust `u64`. Throws on out-of-range input
/// (e.g. negative, or above `u64::MAX`); never wraps.
#[wasm_bindgen]
pub fn bigint_to_u64(v: &BigInt) -> Result<u64, JsValue> {
    // Canonical path: BigInt -> decimal string -> range-checked parse,
    // reusing the host-tested conversion.
    let s = v
        .to_string(10)
        .map(String::from)
        .map_err(|_| js_err("bigint toString failed"))?;
    decimal_str_to_u64(&s).map_err(|e| js_err(e.to_string()))
}

/// Wrap a Rust `i64` as a JS `bigint` (exact over the full range).
#[wasm_bindgen]
pub fn i64_to_bigint(v: i64) -> BigInt {
    BigInt::from(v)
}

/// Convert a JS `bigint` to a Rust `i64`. Throws on out-of-range input.
#[wasm_bindgen]
pub fn bigint_to_i64(v: &BigInt) -> Result<i64, JsValue> {
    let s = v
        .to_string(10)
        .map(String::from)
        .map_err(|_| js_err("bigint toString failed"))?;
    decimal_str_to_i64(&s).map_err(|e| js_err(e.to_string()))
}
