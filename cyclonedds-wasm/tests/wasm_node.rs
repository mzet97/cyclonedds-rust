//! wasm32 client tests, executed under node via `wasm-bindgen-test-runner`.
//!
//! Run: start `node tests/support/ws_echo_server.mjs` (default
//! `ws://127.0.0.1:18711`, override with `WS_ECHO_URL` at compile time),
//! then:
//! `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="wasm-bindgen-test-runner node"`
//! `cargo test -p cyclonedds-wasm --target wasm32-unknown-unknown --test wasm_node`
//!
//! Empty on host (`cargo test`): the wasm32 transport cannot exist there.

#![cfg(target_arch = "wasm32")]

use cyclonedds_wasm::js::{bigint_to_i64, bigint_to_u64, i64_to_bigint, u64_to_bigint, WasmClient};
use js_sys::{Array, BigInt, Function, Reflect};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

fn server_url() -> String {
    option_env!("WS_ECHO_URL")
        .unwrap_or("ws://127.0.0.1:18711/dds")
        .to_string()
}

/// Cooperative sleep via the host event loop (works in node and
/// browsers: no `window` needed, just the global `setTimeout`).
fn sleep_ms(ms: i32) -> js_sys::Promise {
    let global = js_sys::global();
    let set_timeout: Function = Reflect::get(&global, &JsValue::from_str("setTimeout"))
        .expect("setTimeout exists")
        .into();
    js_sys::Promise::new(&mut move |resolve, _| {
        set_timeout
            .call2(&global, &resolve, &JsValue::from(ms))
            .expect("timer armed");
    })
}

/// Wait until `flag` is set or `ms` elapse, yielding every 10 ms so the
/// WebSocket callbacks can run (a busy-wait would starve them).
async fn wait_until(flag: &Rc<Cell<bool>>, ms: f64) -> bool {
    let start = js_sys::Date::now();
    while !flag.get() {
        if js_sys::Date::now() - start > ms {
            return false;
        }
        wasm_bindgen_futures::JsFuture::from(sleep_ms(10))
            .await
            .ok();
    }
    true
}

fn js_str(obj: &JsValue, key: &str) -> String {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default()
}

fn js_num(obj: &JsValue, key: &str) -> f64 {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(f64::NAN)
}

/// The connected client as an opaque JS object: methods are invoked via
/// `Reflect`, exactly like a JS consumer would (the Rust `WasmClient`
/// type cannot be reconstituted from `JsValue` on this side).
async fn connect_client() -> JsValue {
    let promise = WasmClient::connect(server_url(), 5000);
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect("connect resolves against the echo server")
}

fn call_method(obj: &JsValue, name: &str, args: &[JsValue]) -> JsValue {
    let f: Function = Reflect::get(obj, &JsValue::from_str(name))
        .expect("method exists")
        .into();
    match args {
        [] => f.call0(obj).expect("method runs"),
        [a] => f.call1(obj, a).expect("method runs"),
        [a, b, c, d] => f.call4(obj, a, b, c, d).expect("method runs"),
        _ => panic!("arity not covered by this harness"),
    }
}

#[wasm_bindgen_test]
async fn connect_resolves_and_echo_roundtrips() {
    let client = connect_client().await;
    let seen = Rc::new(Cell::new(false));
    let last = Rc::new(RefCell::new((
        String::new(),
        0i32,
        String::new(),
        0f64,
        0usize,
    )));
    let seen_cb = seen.clone();
    let last_cb = last.clone();
    let cb = Closure::wrap(Box::new(move |sample: JsValue| {
        let values = Reflect::get(&sample, &JsValue::from_str("values")).ok();
        let len = values
            .as_ref()
            .and_then(|v| Array::from(v).length().try_into().ok())
            .unwrap_or(0usize);
        *last_cb.borrow_mut() = (
            js_str(&sample, "topic"),
            js_num(&sample, "id") as i32,
            js_str(&sample, "text"),
            js_num(&sample, "seq"),
            len,
        );
        seen_cb.set(true);
    }) as Box<dyn FnMut(JsValue)>);
    call_method(
        &client,
        "on_echo",
        &[cb.as_ref().unchecked_ref::<Function>().clone().into()],
    );
    cb.forget(); // owned by the client for the test lifetime
    let values = Array::new();
    for v in [1, -2, 300] {
        values.push(&JsValue::from(v));
    }
    call_method(
        &client,
        "send_echo",
        &[
            JsValue::from_str("T"),
            JsValue::from(7),
            JsValue::from_str("nóde ✓"),
            values.into(),
        ],
    );
    assert!(wait_until(&seen, 5000.0).await, "echo never arrived");
    assert_eq!(*last.borrow(), ("T".into(), 7, "nóde ✓".into(), 0.0, 3));
    call_method(&client, "close", &[]);
    call_method(&client, "close", &[]); // idempotent, never throws
}

#[wasm_bindgen_test]
async fn dead_port_rejects_with_a_typed_string() {
    let promise = WasmClient::connect("ws://127.0.0.1:9/dds".to_string(), 3000);
    let err = wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .expect_err("dead port must reject");
    let text = err.as_string().unwrap_or_default().to_lowercase();
    assert!(
        text.contains("websocket") || text.contains("closed") || text.contains("timeout"),
        "typed rejection, got: {text}"
    );
}

#[wasm_bindgen_test]
fn bigint_conversions_are_exact_inside_wasm() {
    assert_eq!(bigint_to_u64(&u64_to_bigint(u64::MAX)).unwrap(), u64::MAX);
    assert_eq!(bigint_to_i64(&i64_to_bigint(i64::MIN)).unwrap(), i64::MIN);
    assert!(bigint_to_u64(&BigInt::from(-1i64)).is_err());
    assert!(bigint_to_i64(&BigInt::from(u64::MAX)).is_err());
}
