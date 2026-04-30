use cyclonedds::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static TOPIC_COUNTER: AtomicU64 = AtomicU64::new(1);

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TestMessage {
    pub id: i32,
    pub value: i32,
    pub text: [u8; 64],
}

impl TestMessage {
    pub fn new(id: i32, value: i32, text: &str) -> Self {
        let mut bytes = [0u8; 64];
        let encoded = text.as_bytes();
        let len = encoded.len().min(bytes.len().saturating_sub(1));
        bytes[..len].copy_from_slice(&encoded[..len]);
        Self {
            id,
            value,
            text: bytes,
        }
    }

    pub fn text(&self) -> String {
        let end = self
            .text
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(self.text.len());
        String::from_utf8_lossy(&self.text[..end]).into_owned()
    }
}

impl DdsType for TestMessage {
    fn type_name() -> &'static str {
        "CycloneRustTestMessage"
    }

    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr(
            TYPE_4BY | OP_FLAG_SGN,
            std::mem::offset_of!(TestMessage, id) as u32,
        ));
        ops.extend(adr(
            TYPE_4BY | OP_FLAG_SGN,
            std::mem::offset_of!(TestMessage, value) as u32,
        ));
        ops.extend(adr_bst(std::mem::offset_of!(TestMessage, text) as u32, 64));
        ops
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KeyedMessage {
    pub key: i32,
    pub value: i32,
}

impl DdsType for KeyedMessage {
    fn type_name() -> &'static str {
        "CycloneRustKeyedMessage"
    }

    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr_key(
            TYPE_4BY | OP_FLAG_SGN,
            std::mem::offset_of!(KeyedMessage, key) as u32,
        ));
        ops.extend(adr(
            TYPE_4BY | OP_FLAG_SGN,
            std::mem::offset_of!(KeyedMessage, value) as u32,
        ));
        ops
    }

    fn key_count() -> usize {
        1
    }

    fn keys() -> Vec<KeyDescriptor> {
        vec![KeyDescriptor {
            name: "key".into(),
            ops_path: vec![0],
        }]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, DdsTypeDerive)]
pub struct DerivedReading {
    #[key]
    pub sensor_id: u32,
    pub temperature_raw: i32,
    pub humidity_raw: i32,
}

pub fn unique_topic(prefix: &str) -> String {
    let seq = TOPIC_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}_{}_{}", prefix, std::process::id(), seq)
}

pub fn wait_for<F>(timeout: Duration, mut predicate: F) -> bool
where
    F: FnMut() -> bool,
{
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    predicate()
}

pub fn short_delay() {
    std::thread::sleep(Duration::from_millis(200));
}
