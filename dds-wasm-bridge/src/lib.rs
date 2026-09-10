//! Native reference gateway for the browser-DDS bridge (Phase C).
//!
//! First end-to-end slice: one IDL type with key + string + sequence
//! ([`WasmEcho`]) travels as binary CDR ([`DataFrame`] `FLAG_CDR_LE`, codec
//! in `cyclonedds_proto::echo`) between a browser-like client and this
//! gateway, which translates to/from a native DDS peer. Both directions.
//!
//! Transport note: the TCP framing here (`u32 LE length` + payload) carries
//! the *identical* [`DataFrame`] bytes the WebSocket transport carries —
//! only the socket type differs, so a browser cannot run inside `cargo
//! test`. The wasm32 client (`cyclonedds_wasm::WasmEchoWriter`, binary
//! `ArrayBuffer` send) speaks the same frames; the host test peer
//! ([`BridgeClient`]) replays them over TCP.
//!
//! Legacy JSON (`{"topic","data"}`) is accepted **only** when the gateway
//! is built with `legacy_json: true` (explicit compat). Otherwise it is
//! rejected with a typed `error` control reply and the connection stays
//! open. Malformed frames behave the same way: typed `error` reply, no
//! teardown, no panic.
//!
//! Not yet implemented (recorded gaps, not discretized): `hello` handshake
//! and `ack`s, multi-topic routing (one topic per gateway), reliable QoS
//! (anything but best-effort/volatile is `Unsupported`), WebSocket framing
//! on the gateway side.

use cyclonedds::{DdsSequence, DdsTypeDerive};
use cyclonedds_proto::{
    check_qos,
    echo::{decode_echo_xcdr1, encode_echo_xcdr1, EchoMsg},
    Control, DataFrame, LegacyEnvelope, ProtoError, FLAG_CDR_LE, MAX_FRAME_BYTES, PROTO_VERSION,
};
use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Typed gateway/client error. Timeouts and disconnects are first-class so
/// tests can assert on them instead of hanging.
#[derive(Debug)]
pub enum BridgeError {
    Io(io::Error),
    Proto(ProtoError),
    Dds(String),
    /// A bounded wait produced no data. Observable, not a hang.
    Timeout,
    /// Wait cancelled via [`CancelFlag`]. Observable, not a hang.
    Cancelled,
    /// Peer closed the connection cleanly.
    Disconnected,
    UnexpectedShape(String),
}

impl std::fmt::Display for BridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BridgeError::Io(e) => write!(f, "io: {e}"),
            BridgeError::Proto(e) => write!(f, "proto: {e}"),
            BridgeError::Dds(e) => write!(f, "dds: {e}"),
            BridgeError::Timeout => write!(f, "timed out"),
            BridgeError::Cancelled => write!(f, "cancelled"),
            BridgeError::Disconnected => write!(f, "disconnected"),
            BridgeError::UnexpectedShape(e) => write!(f, "unexpected: {e}"),
        }
    }
}

impl std::error::Error for BridgeError {}

impl From<io::Error> for BridgeError {
    fn from(e: io::Error) -> Self {
        BridgeError::Io(e)
    }
}

impl From<ProtoError> for BridgeError {
    fn from(e: ProtoError) -> Self {
        BridgeError::Proto(e)
    }
}

pub type BridgeResult<T> = Result<T, BridgeError>;

// ---------------------------------------------------------------------------
// Demo IDL type: key + string + sequence
//
// IDL: `struct WasmEcho { @key long id; string text; sequence<long> values; };`
// ---------------------------------------------------------------------------

/// Native DDS representation of the demo type.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, DdsTypeDerive)]
pub struct WasmEcho {
    #[key]
    pub id: i32,
    pub text: String,
    pub values: DdsSequence<i32>,
}

impl WasmEcho {
    /// Portable -> native (validates lengths via the DDS allocator).
    pub fn from_proto(msg: &EchoMsg) -> BridgeResult<Self> {
        Ok(WasmEcho {
            id: msg.id,
            text: msg.text.clone(),
            values: DdsSequence::from_vec(msg.values.clone())
                .map_err(|e| BridgeError::Dds(e.to_string()))?,
        })
    }

    /// Native -> portable.
    pub fn to_proto(&self) -> EchoMsg {
        EchoMsg {
            id: self.id,
            text: self.text.clone(),
            values: self.values.to_vec(),
        }
    }
}

// ---------------------------------------------------------------------------
// Config + stats
// ---------------------------------------------------------------------------

/// Gateway configuration.
#[derive(Debug, Clone)]
pub struct BridgeConfig {
    /// Primary DDS topic served by this gateway.
    pub topic: String,
    /// Extra DDS topics served by the same gateway (Phase D: one gateway,
    /// one DDS owner thread, N readers/writers). Empty by default, which
    /// preserves the Phase C single-topic behaviour.
    pub extra_topics: Vec<String>,
    /// Accept legacy JSON envelopes (`{"topic","data"}`) as explicit
    /// compat. Default `false`: JSON is rejected with an `error` reply.
    pub legacy_json: bool,
    /// Per-connection bounded queue capacity for DDS->client frames
    /// (Phase D). When full, the oldest behaviour is explicit: the new
    /// frame is dropped and `dropped_overflow` is incremented — never a
    /// silent unbounded grow, never a block of the DDS owner thread.
    pub queue_cap: usize,
}

/// Default per-connection queue depth.
pub const DEFAULT_QUEUE_CAP: usize = 64;

/// Longest topic name the gateway accepts. Wire frames cap at
/// `MAX_FRAME_BYTES` and DDS discovery carries the name on every
/// endpoint, so anything past this cannot round-trip — and native
/// `dds_create_topic` aborts the process (SIGSEGV) instead of failing
/// on multi-megabyte names. Fail fast with a typed error instead.
pub const MAX_TOPIC_CHARS: usize = 1024;

impl Default for BridgeConfig {
    fn default() -> Self {
        BridgeConfig {
            topic: cyclonedds_proto::echo::ECHO_TOPIC.into(),
            extra_topics: Vec::new(),
            legacy_json: false,
            queue_cap: DEFAULT_QUEUE_CAP,
        }
    }
}

impl BridgeConfig {
    /// Every topic this gateway serves (primary first).
    pub fn topics(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(1 + self.extra_topics.len());
        out.push(self.topic.clone());
        out.extend(self.extra_topics.iter().cloned());
        out
    }
}

/// Observable counters (best-effort data plane + typed rejections).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BridgeStats {
    pub frames_in: u64,
    pub samples_in: u64,
    pub samples_out: u64,
    pub errors_out: u64,
    pub seq_gaps: u64,
    /// Frames dropped because a per-connection bounded queue was full.
    /// Explicit overflow accounting (Phase D): grows only via `try_send`
    /// failure, never via blocking or unbounded buffering.
    pub dropped_overflow: u64,
}

#[derive(Debug, Default)]
struct StatsInner {
    frames_in: AtomicU64,
    samples_in: AtomicU64,
    samples_out: AtomicU64,
    errors_out: AtomicU64,
    seq_gaps: AtomicU64,
    dropped_overflow: AtomicU64,
}

impl StatsInner {
    fn snapshot(&self) -> BridgeStats {
        BridgeStats {
            frames_in: self.frames_in.load(Ordering::Relaxed),
            samples_in: self.samples_in.load(Ordering::Relaxed),
            samples_out: self.samples_out.load(Ordering::Relaxed),
            errors_out: self.errors_out.load(Ordering::Relaxed),
            seq_gaps: self.seq_gaps.load(Ordering::Relaxed),
            dropped_overflow: self.dropped_overflow.load(Ordering::Relaxed),
        }
    }
}

// ---------------------------------------------------------------------------
// Length-prefixed framing: u32 LE length + payload (payload = DataFrame
// bytes or, in explicit compat mode, legacy JSON text).
// ---------------------------------------------------------------------------

fn read_packet(sock: &mut TcpStream) -> BridgeResult<Vec<u8>> {
    // A read timeout surfaces as `WouldBlock` on this platform: map it to
    // the typed `Timeout` so waits stay observable instead of surfacing
    // as a raw OS error.
    let map_err = |e: io::Error| match e.kind() {
        io::ErrorKind::UnexpectedEof => BridgeError::Disconnected,
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut => BridgeError::Timeout,
        _ => BridgeError::Io(e),
    };
    let mut len_buf = [0u8; 4];
    sock.read_exact(&mut len_buf).map_err(map_err)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_FRAME_BYTES + 64 {
        return Err(BridgeError::Proto(ProtoError::TooLarge {
            got: len,
            max: MAX_FRAME_BYTES + 64,
        }));
    }
    let mut payload = vec![0u8; len];
    sock.read_exact(&mut payload).map_err(map_err)?;
    Ok(payload)
}

fn write_packet(sock: &mut TcpStream, payload: &[u8]) -> io::Result<()> {
    sock.write_all(&(payload.len() as u32).to_le_bytes())?;
    sock.write_all(payload)?;
    sock.flush()
}

fn error_packet(code: &str, detail: &str) -> Vec<u8> {
    let ctrl = Control::Error {
        proto: PROTO_VERSION,
        code: code.into(),
        detail: detail.into(),
    };
    let mut out = Vec::new();
    let json = ctrl
        .to_json()
        .unwrap_or_else(|_| r#"{"proto":0,"kind":"error","code":"internal","detail":"?"}"#.into());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(json.as_bytes());
    out
}

/// Try to parse a length-prefixed payload as an `error` control reply.
pub fn parse_error_reply(payload: &[u8]) -> Option<(String, String)> {
    let s = std::str::from_utf8(payload).ok()?;
    match Control::from_json(s).ok()? {
        Control::Error { code, detail, .. } => Some((code, detail)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Cancellation + reconnect backoff (Phase D)
// ---------------------------------------------------------------------------

/// Cooperative cancellation token for blocking client waits.
///
/// Cloned cheaply across threads; `cancel()` wakes any
/// [`BridgeClient::recv_echo_cancel`] wait at the next 200 ms quantum with
/// `BridgeError::Cancelled`. Dropping the token without cancelling never
/// cancels.
#[derive(Debug, Clone, Default)]
pub struct CancelFlag(Arc<AtomicBool>);

impl CancelFlag {
    pub fn new() -> Self {
        CancelFlag(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// Exponential backoff delay for reconnect attempts: `base * 2^attempt`
/// capped at `cap` (saturating, pure function so tests assert the schedule
/// without sleeping).
pub fn backoff_delay(attempt: u32, base: Duration, cap: Duration) -> Duration {
    let mut d = base;
    for _ in 0..attempt {
        d = d.saturating_mul(2);
        if d >= cap {
            return cap;
        }
    }
    d.min(cap)
}

// ---------------------------------------------------------------------------
// Gateway
// ---------------------------------------------------------------------------

enum DdsCmd {
    Publish { topic: String, sample: WasmEcho },
}

/// Wait for the DDS owner thread's readiness report. Split out so the
/// timeout arm is unit-testable without spawning threads.
fn await_dds_ready(rx: mpsc::Receiver<BridgeResult<()>>, timeout: Duration) -> BridgeResult<()> {
    match rx.recv_timeout(timeout) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(BridgeError::Dds(
            "dds thread did not report readiness in time".into(),
        )),
    }
}

/// Forward one translated sample to the DDS owner thread. Returns `false`
/// when the owner is gone (shutdown): callers close the connection instead
/// of spinning on a doomed send.
fn publish_to_dds(
    dds_tx: &mpsc::Sender<DdsCmd>,
    topic: String,
    sample: WasmEcho,
    stats: &Arc<StatsInner>,
) -> bool {
    if dds_tx.send(DdsCmd::Publish { topic, sample }).is_err() {
        return false;
    }
    stats.samples_in.fetch_add(1, Ordering::Relaxed);
    true
}

/// One connected client as seen by the DDS owner thread: a bounded queue
/// plus the topic subscription set its dispatcher manages.
struct ClientSlot {
    tx: mpsc::SyncSender<Vec<u8>>,
    subs: Mutex<HashSet<String>>,
}

type Registry = Arc<Mutex<HashMap<u64, ClientSlot>>>;

/// Native reference gateway. Owns no DDS entity on this thread: a single
/// background thread owns participant/writers/readers; connection threads
/// only parse frames and forward commands.
pub struct WasmBridge {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    stats: Arc<StatsInner>,
    /// `Mutex` so `close()` (via `&self`) can drop it: a dead gateway
    /// must refuse new connections, not leave them hanging in the
    /// kernel backlog with no accept thread to serve them.
    listener: Mutex<Option<TcpListener>>,
    /// Shutdown handles for live connections: `close()` uses them to
    /// unblock connection threads parked in `read`, so no thread
    /// outlives the bridge waiting on a dead socket.
    conns: Arc<Mutex<HashMap<u64, TcpStream>>>,
    /// Per-connection queue slots: `close()` clears them (dropping every
    /// queue sender) so pump threads exit even if a client never hangs up.
    registry: Registry,
}

impl WasmBridge {
    /// Bind `127.0.0.1:0`, start the DDS thread + accept loop, wait until
    /// the DDS entities are up (or fail fast with the DDS error).
    pub fn bind(config: BridgeConfig) -> BridgeResult<Arc<Self>> {
        for name in std::iter::once(&config.topic).chain(config.extra_topics.iter()) {
            if name.len() > MAX_TOPIC_CHARS {
                return Err(BridgeError::Dds(format!(
                    "topic name too long ({} chars, max {MAX_TOPIC_CHARS})",
                    name.len()
                )));
            }
        }
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let stats = Arc::new(StatsInner::default());
        let registry: Registry = Arc::new(Mutex::new(HashMap::new()));
        let conns: Arc<Mutex<HashMap<u64, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));
        let next_id = Arc::new(AtomicU64::new(1));
        let (dds_tx, dds_rx) = mpsc::channel::<DdsCmd>();
        let (ready_tx, ready_rx) = mpsc::channel::<BridgeResult<()>>();

        // Single DDS owner thread: entities never cross threads.
        {
            let config = config.clone();
            let registry = registry.clone();
            let shutdown = shutdown.clone();
            let stats = stats.clone();
            std::thread::Builder::new()
                .name("dds-wasm-bridge-dds".into())
                .spawn(move || {
                    dds_loop(config, registry, dds_rx, ready_tx, shutdown, stats);
                })
                .map_err(BridgeError::Io)?;
        }
        // Fail fast when DDS setup breaks (sandbox without loopback, ...).
        await_dds_ready(ready_rx, Duration::from_secs(30))?;

        let bridge = Arc::new(WasmBridge {
            addr,
            shutdown,
            stats,
            listener: Mutex::new(Some(listener)),
            conns: conns.clone(),
            registry: registry.clone(),
        });

        // Accept loop (detached; exits when the listener is dropped).
        // `config`/`registry` travel in this closure: no process-global
        // state, so several gateways can coexist in one test binary.
        {
            let shutdown = bridge.shutdown.clone();
            let stats = bridge.stats.clone();
            let listener = bridge
                .listener
                .lock()
                .expect("listener")
                .as_ref()
                .expect("listener present")
                .try_clone()
                .map_err(BridgeError::Io)?;
            let conn_config = config.clone();
            let next_id = next_id.clone();
            let conns = bridge.conns.clone();
            listener.set_nonblocking(true).map_err(BridgeError::Io)?;
            std::thread::Builder::new()
                .name("dds-wasm-bridge-accept".into())
                .spawn(move || loop {
                    if shutdown.load(Ordering::Relaxed) {
                        break;
                    }
                    match listener.accept() {
                        Ok((sock, _)) => {
                            let id = next_id.fetch_add(1, Ordering::Relaxed);
                            serve_conn(
                                &conn_config,
                                &stats,
                                sock,
                                id,
                                dds_tx.clone(),
                                registry.clone(),
                                &conns,
                            );
                        }
                        // WouldBlock (idle) and unexpected errors (e.g.
                        // EMFILE) share the backoff: shutdown is observed
                        // at the loop top, so no busy spin on a
                        // persistently failing accept.
                        Err(_) => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                    }
                })
                .map_err(BridgeError::Io)?;
        }
        Ok(bridge)
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn stats(&self) -> BridgeStats {
        self.stats.snapshot()
    }

    /// Signal every thread owned by this bridge to stop. Idempotent.
    /// Live connections are shut down (their threads observe EOF and
    /// exit; queue senders are dropped so pumps exit too). [`Drop`]
    /// calls this; tests use it to assert clean shutdown without
    /// relying on drop order.
    pub fn close(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
        for (_, sock) in self.conns.lock().expect("conns").drain() {
            let _ = sock.shutdown(std::net::Shutdown::Both);
        }
        // Dropping every queue sender lets pump threads exit; connection
        // threads remove their own slot (idempotent) as they unwind.
        self.registry.lock().expect("registry").clear();
        // Dropping the listener makes new connects fail fast instead of
        // completing in the kernel backlog with nobody to serve them.
        drop(self.listener.lock().expect("listener").take());
    }
}

/// Send one versioned control reply on this connection.
fn send_control(sock: &mut TcpStream, ctrl: &Control) {
    let json = ctrl
        .to_json()
        .unwrap_or_else(|_| r#"{"proto":0,"kind":"error","code":"internal","detail":"?"}"#.into());
    let _ = write_packet(sock, json.as_bytes());
}

/// Per-connection dispatcher action after one `{`-prefixed packet.
enum ConnAction {
    Keep,
    Bye,
}

/// Handle one `{`-prefixed packet: versioned control plane (`"kind"`
/// present) or the legacy JSON envelope path (explicit compat only).
/// Pure dispatch on already-read bytes; every rejection is a typed `error`
/// reply, never a teardown (except `bye`, which is a clean close).
#[allow(clippy::too_many_arguments)]
fn handle_json_packet(
    sock: &mut TcpStream,
    payload: &[u8],
    served: &HashSet<String>,
    legacy_json: bool,
    registry: &Registry,
    id: u64,
    stats: &Arc<StatsInner>,
    dds_tx: &mpsc::Sender<DdsCmd>,
) -> ConnAction {
    // Control plane first: any JSON object carrying `"kind"` is a control
    // message, independent of the legacy-compat flag.
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(payload) {
        if value.get("kind").is_some() {
            // `from_slice` above already proved valid UTF-8 (serde_json
            // rejects anything else), so this cannot fail on reachable
            // inputs; the fallback keeps the function total.
            let text = std::str::from_utf8(payload).unwrap_or("");
            let ctrl = match Control::from_json(text) {
                Ok(c) => c,
                Err(e) => {
                    stats.errors_out.fetch_add(1, Ordering::Relaxed);
                    let pkt = error_packet("invalid_frame", &e.to_string());
                    let _ = sock.write_all(&pkt);
                    return ConnAction::Keep;
                }
            };
            if let Err(e) = ctrl.check_version() {
                stats.errors_out.fetch_add(1, Ordering::Relaxed);
                let pkt = error_packet("unsupported_proto", &e.to_string());
                let _ = sock.write_all(&pkt);
                return ConnAction::Keep;
            }
            match ctrl {
                Control::Hello { .. } => {
                    send_control(
                        sock,
                        &Control::Ack {
                            proto: PROTO_VERSION,
                            seq: 0,
                        },
                    );
                }
                Control::Register {
                    topic, qos, seq, ..
                } => {
                    if !served.contains(&topic) {
                        stats.errors_out.fetch_add(1, Ordering::Relaxed);
                        let pkt = error_packet(
                            "unknown_topic",
                            &format!("no such topic (serving {})", served.len()),
                        );
                        let _ = sock.write_all(&pkt);
                    } else if let Err(e) = check_qos(&qos) {
                        // QoS incompatível falha alto: audível, tipado.
                        stats.errors_out.fetch_add(1, Ordering::Relaxed);
                        let pkt = error_packet("unsupported_qos", &e.to_string());
                        let _ = sock.write_all(&pkt);
                    } else {
                        if let Some(slot) = registry.lock().expect("registry").get(&id) {
                            slot.subs.lock().expect("subs").insert(topic);
                        }
                        send_control(
                            sock,
                            &Control::Ack {
                                proto: PROTO_VERSION,
                                seq,
                            },
                        );
                    }
                }
                Control::Subscribe { topic, seq, .. } => {
                    if !served.contains(&topic) {
                        stats.errors_out.fetch_add(1, Ordering::Relaxed);
                        let pkt = error_packet(
                            "unknown_topic",
                            &format!("no such topic (serving {})", served.len()),
                        );
                        let _ = sock.write_all(&pkt);
                    } else {
                        if let Some(slot) = registry.lock().expect("registry").get(&id) {
                            slot.subs.lock().expect("subs").insert(topic);
                        }
                        send_control(
                            sock,
                            &Control::Ack {
                                proto: PROTO_VERSION,
                                seq,
                            },
                        );
                    }
                }
                Control::Unsubscribe { topic, seq, .. } => {
                    if let Some(slot) = registry.lock().expect("registry").get(&id) {
                        slot.subs.lock().expect("subs").remove(&topic);
                    }
                    send_control(
                        sock,
                        &Control::Ack {
                            proto: PROTO_VERSION,
                            seq,
                        },
                    );
                }
                Control::Bye { .. } => return ConnAction::Bye,
                Control::Ack { .. } | Control::Error { .. } => {}
            }
            return ConnAction::Keep;
        }
    }
    // Legacy JSON text path: explicit compat only.
    if !legacy_json {
        stats.errors_out.fetch_add(1, Ordering::Relaxed);
        let pkt = error_packet(
            "legacy_json_disabled",
            "JSON data plane requires explicit compat mode",
        );
        let _ = sock.write_all(&pkt);
        return ConnAction::Keep;
    }
    let text = match std::str::from_utf8(payload) {
        Ok(t) => t,
        Err(_) => {
            stats.errors_out.fetch_add(1, Ordering::Relaxed);
            let pkt = error_packet("invalid_frame", "legacy payload not utf-8");
            let _ = sock.write_all(&pkt);
            return ConnAction::Keep;
        }
    };
    match LegacyEnvelope::decode(text) {
        Ok(env) if served.contains(&env.topic) => {
            match serde_json::from_value::<EchoMsg>(env.data) {
                Ok(msg) => match WasmEcho::from_proto(&msg) {
                    Ok(native) => {
                        if !publish_to_dds(dds_tx, env.topic, native, stats) {
                            return ConnAction::Bye;
                        }
                    }
                    Err(e) => {
                        stats.errors_out.fetch_add(1, Ordering::Relaxed);
                        let pkt = error_packet("serialization", &format!("native convert: {e}"));
                        let _ = sock.write_all(&pkt);
                    }
                },
                Err(e) => {
                    stats.errors_out.fetch_add(1, Ordering::Relaxed);
                    let pkt = error_packet("serialization", &format!("bad json: {e}"));
                    let _ = sock.write_all(&pkt);
                }
            }
        }
        _ => {
            stats.errors_out.fetch_add(1, Ordering::Relaxed);
            let pkt = error_packet(
                "unknown_topic",
                &format!("no such topic (serving {} topics)", served.len()),
            );
            let _ = sock.write_all(&pkt);
        }
    }
    ConnAction::Keep
}

/// One connection: a dispatcher pair (Phase D).
///
/// * reader loop (this thread's child): parses length-prefixed packets,
///   translates data frames to [`DdsCmd`], and owns the per-connection
///   subscription set (`subscribe`/`unsubscribe`/`register`). Never panics
///   on adversarial input; every rejection is a typed `error` reply.
/// * pump loop: pushes DDS->client frames from the connection's **bounded**
///   queue to the socket. Overflow is explicit (`dropped_overflow`), never
///   a block of the DDS owner thread.
///
/// Dropping either side unregisters the slot, which drops the queue sender
/// and lets the other side exit: `close()`/connection loss leaves no
/// orphan thread holding the socket.
fn serve_conn(
    config: &BridgeConfig,
    stats: &Arc<StatsInner>,
    sock: TcpStream,
    id: u64,
    dds_tx: mpsc::Sender<DdsCmd>,
    registry: Registry,
    conns: &Arc<Mutex<HashMap<u64, TcpStream>>>,
) {
    // Accepted sockets inherit the listener's non-blocking mode on Windows
    // (Linux clears it on accept). The reader/pump loops below assume
    // blocking IO: on a non-blocking socket the first empty read surfaces
    // WouldBlock, which the reader maps to an exit, dropping every
    // connection right after serving its first packet(s).
    if sock.set_nonblocking(false).is_err() {
        return;
    }
    let served: HashSet<String> = config.topics().into_iter().collect();
    let queue_cap = config.queue_cap.max(1);
    let legacy_json = config.legacy_json;

    // Shutdown handle for `close()`: unblocks this connection's reader.
    if let Ok(shutdown_sock) = sock.try_clone() {
        conns.lock().expect("conns").insert(id, shutdown_sock);
    }

    // Bounded DDS->client queue; default subscription is every served topic.
    let (qtx, qrx) = mpsc::sync_channel::<Vec<u8>>(queue_cap);
    registry.lock().expect("registry").insert(
        id,
        ClientSlot {
            tx: qtx,
            subs: Mutex::new(served.clone()),
        },
    );

    // Pump: bounded queue -> socket (owns a clone; the reader loop owns the
    // original for reads + control replies).
    match sock.try_clone() {
        Ok(pump_sock) => {
            let registry = registry.clone();
            std::thread::Builder::new()
                .name("dds-wasm-bridge-pump".into())
                .spawn(move || {
                    let mut out = pump_sock;
                    while let Ok(bytes) = qrx.recv() {
                        if write_packet(&mut out, &bytes).is_err() {
                            break;
                        }
                    }
                    registry.lock().expect("registry").remove(&id);
                })
                .expect("pump thread");
        }
        Err(_) => {
            registry.lock().expect("registry").remove(&id);
            conns.lock().expect("conns").remove(&id);
            return;
        }
    }

    let stats = stats.clone();
    let conns = conns.clone();
    std::thread::Builder::new()
        .name("dds-wasm-bridge-conn".into())
        .spawn(move || {
            let mut sock = sock;
            let mut last_seq: HashMap<String, u32> = HashMap::new();
            loop {
                let payload = match read_packet(&mut sock) {
                    Ok(p) => p,
                    Err(BridgeError::Disconnected) => break,
                    Err(_) => break,
                };
                stats.frames_in.fetch_add(1, Ordering::Relaxed);
                if payload.first() == Some(&b'{') {
                    match handle_json_packet(
                        &mut sock,
                        &payload,
                        &served,
                        legacy_json,
                        &registry,
                        id,
                        &stats,
                        &dds_tx,
                    ) {
                        ConnAction::Keep => continue,
                        ConnAction::Bye => break,
                    }
                }
                // Binary CDR path.
                let frame = match DataFrame::decode(&payload) {
                    Ok(f) => f,
                    Err(e) => {
                        stats.errors_out.fetch_add(1, Ordering::Relaxed);
                        let code = match &e {
                            ProtoError::Unsupported(_) => "unsupported_proto",
                            ProtoError::TooLarge { .. } => "frame_too_large",
                            _ => "invalid_frame",
                        };
                        let pkt = error_packet(code, &e.to_string());
                        let _ = sock.write_all(&pkt);
                        continue;
                    }
                };
                if frame.flags != FLAG_CDR_LE {
                    stats.errors_out.fetch_add(1, Ordering::Relaxed);
                    let pkt = error_packet(
                        "unsupported_flags",
                        "echo data plane requires CDR-LE frames",
                    );
                    let _ = sock.write_all(&pkt);
                    continue;
                }
                if !served.contains(&frame.topic) {
                    stats.errors_out.fetch_add(1, Ordering::Relaxed);
                    let pkt = error_packet(
                        "unknown_topic",
                        &format!("no such topic (serving {} topics)", served.len()),
                    );
                    let _ = sock.write_all(&pkt);
                    continue;
                }
                match last_seq.get(&frame.topic) {
                    Some(prev) if frame.seq != prev.wrapping_add(1) => {
                        stats.seq_gaps.fetch_add(1, Ordering::Relaxed);
                    }
                    _ => {}
                }
                last_seq.insert(frame.topic.clone(), frame.seq);
                match decode_echo_xcdr1(&frame.cdr) {
                    Ok(msg) => match WasmEcho::from_proto(&msg) {
                        Ok(native) => {
                            if !publish_to_dds(&dds_tx, frame.topic, native, &stats) {
                                break;
                            }
                        }
                        Err(e) => {
                            stats.errors_out.fetch_add(1, Ordering::Relaxed);
                            let pkt =
                                error_packet("serialization", &format!("native convert: {e}"));
                            let _ = sock.write_all(&pkt);
                        }
                    },
                    Err(e) => {
                        stats.errors_out.fetch_add(1, Ordering::Relaxed);
                        let pkt = error_packet("serialization", &e.to_string());
                        let _ = sock.write_all(&pkt);
                    }
                }
            }
            // Unregister: drops the queue sender, so the pump loop exits.
            // No leaked thread, no leaked socket half.
            registry.lock().expect("registry").remove(&id);
            conns.lock().expect("conns").remove(&id);
        })
        .expect("conn thread");
}

impl Drop for WasmBridge {
    fn drop(&mut self) {
        // Full shutdown (listener, connections, pumps, DDS thread).
        self.close();
    }
}

// ---------------------------------------------------------------------------
// DDS owner thread
// ---------------------------------------------------------------------------

/// Owns participant/writers/readers for the gateway's whole lifetime (one
/// lane per served topic, Phase D). Reports readiness through `ready_tx`,
/// then pumps both directions: channel commands -> the lane writer for
/// `cmd.topic`, lane takes -> routed frames to subscribed TCP clients.
fn dds_loop(
    config: BridgeConfig,
    registry: Registry,
    dds_rx: mpsc::Receiver<DdsCmd>,
    ready_tx: mpsc::Sender<BridgeResult<()>>,
    shutdown: Arc<AtomicBool>,
    stats: Arc<StatsInner>,
) {
    struct Lane {
        topic: String,
        writer: cyclonedds::DataWriter<WasmEcho>,
        reader: cyclonedds::DataReader<WasmEcho>,
    }
    type SetupBundle = (
        cyclonedds::DomainParticipant,
        cyclonedds::Publisher,
        cyclonedds::Subscriber,
        Vec<cyclonedds::Topic<WasmEcho>>,
        Vec<Lane>,
    );
    let setup = (|| -> BridgeResult<SetupBundle> {
        let participant =
            cyclonedds::DomainParticipant::new(0).map_err(|e| BridgeError::Dds(e.to_string()))?;
        let publisher = participant
            .create_publisher()
            .map_err(|e| BridgeError::Dds(e.to_string()))?;
        let subscriber = participant
            .create_subscriber()
            .map_err(|e| BridgeError::Dds(e.to_string()))?;
        let mut topics = Vec::new();
        let mut lanes = Vec::new();
        for name in config.topics() {
            let topic = participant
                .create_topic::<WasmEcho>(&name)
                .map_err(|e| BridgeError::Dds(e.to_string()))?;
            let writer = publisher
                .create_writer(&topic)
                .map_err(|e| BridgeError::Dds(e.to_string()))?;
            let reader = subscriber
                .create_reader(&topic)
                .map_err(|e| BridgeError::Dds(e.to_string()))?;
            topics.push(topic);
            lanes.push(Lane {
                topic: name,
                writer,
                reader,
            });
        }
        Ok((participant, publisher, subscriber, topics, lanes))
    })();
    // Parent entities stay alive as locals for the thread's whole lifetime.
    let (_participant, _publisher, _subscriber, _topics, lanes) = match setup {
        Ok(bundle) => {
            let _ = ready_tx.send(Ok(()));
            bundle
        }
        Err(e) => {
            let _ = ready_tx.send(Err(BridgeError::Dds(e.to_string())));
            return;
        }
    };
    let writers: HashMap<&str, &cyclonedds::DataWriter<WasmEcho>> = lanes
        .iter()
        .map(|l| (l.topic.as_str(), &l.writer))
        .collect();

    let mut out_seq: u32 = 0;
    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }
        // Client -> DDS: route each command to its topic lane.
        while let Ok(cmd) = dds_rx.try_recv() {
            match cmd {
                DdsCmd::Publish { topic, sample } => {
                    if let Some(w) = writers.get(topic.as_str()) {
                        if w.write(&sample).is_err() {
                            break;
                        }
                    }
                }
            }
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
        }
        // DDS -> clients: typed take per lane, re-encode as binary CDR,
        // route only to subscribed connections.
        for lane in &lanes {
            if let Ok(samples) = lane.reader.take() {
                for sample in samples {
                    let msg = sample.to_proto();
                    let cdr = match encode_echo_xcdr1(&msg) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };
                    let frame = DataFrame {
                        proto: PROTO_VERSION,
                        flags: FLAG_CDR_LE,
                        topic: lane.topic.clone(),
                        cdr,
                        seq: out_seq,
                    };
                    out_seq = out_seq.wrapping_add(1);
                    let bytes = match frame.encode() {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    route_to_subscribers(&registry, &stats, &lane.topic, &bytes);
                    stats.samples_out.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Route one DDS->client frame to subscribed connections only.
///
/// Non-blocking (`try_send`): a full bounded queue drops the frame and
/// bumps `dropped_overflow` — explicit overflow, never a stall of this
/// thread. Dead queues (`Disconnected`) are pruned from the registry.
fn route_to_subscribers(registry: &Registry, stats: &Arc<StatsInner>, topic: &str, payload: &[u8]) {
    let targets: Vec<(u64, mpsc::SyncSender<Vec<u8>>)> = {
        let guard = registry.lock().expect("registry");
        guard
            .iter()
            .filter(|(_, slot)| slot.subs.lock().expect("subs").contains(topic))
            .map(|(id, slot)| (*id, slot.tx.clone()))
            .collect()
    };
    for (id, tx) in targets {
        match tx.try_send(payload.to_vec()) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(_)) => {
                stats.dropped_overflow.fetch_add(1, Ordering::Relaxed);
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                registry.lock().expect("registry").remove(&id);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Host test client: replays the browser binary protocol over TCP.
// ---------------------------------------------------------------------------

/// Browser-stand-in: speaks the same length-prefixed [`DataFrame`] bytes
/// the wasm32 `WasmEchoWriter` emits, over TCP instead of WebSocket.
#[derive(Debug)]
pub struct BridgeClient {
    sock: TcpStream,
    seq: u32,
}

impl BridgeClient {
    pub fn connect(addr: SocketAddr) -> BridgeResult<Self> {
        let sock = TcpStream::connect(addr)?;
        Ok(BridgeClient { sock, seq: 0 })
    }

    /// Connect with exponential-backoff retries: `attempts` extra tries
    /// after the first failure, spaced by
    /// [`backoff_delay`](backoff_delay)`(n, base, cap)`. Fails loud with
    /// the last `io` error when the budget is exhausted.
    pub fn connect_with_retry(
        addr: SocketAddr,
        attempts: u32,
        base: Duration,
        cap: Duration,
    ) -> BridgeResult<Self> {
        let mut attempt: u32 = 0;
        loop {
            match TcpStream::connect(addr) {
                Ok(sock) => return Ok(BridgeClient { sock, seq: 0 }),
                Err(e) => {
                    if attempt >= attempts {
                        return Err(BridgeError::Io(e));
                    }
                    std::thread::sleep(backoff_delay(attempt, base, cap));
                    attempt += 1;
                }
            }
        }
    }

    /// Send one sample as a binary CDR frame (client -> gateway -> DDS).
    pub fn send_echo(&mut self, topic: &str, msg: &EchoMsg) -> BridgeResult<()> {
        let cdr = encode_echo_xcdr1(msg)?;
        let frame = DataFrame {
            proto: PROTO_VERSION,
            flags: FLAG_CDR_LE,
            topic: topic.into(),
            cdr,
            seq: self.seq,
        };
        self.seq = self.seq.wrapping_add(1);
        let bytes = frame.encode()?;
        write_packet(&mut self.sock, &bytes)?;
        Ok(())
    }

    /// Send a legacy JSON envelope (compat path only).
    pub fn send_legacy(&mut self, topic: &str, msg: &EchoMsg) -> BridgeResult<()> {
        let data =
            serde_json::to_value(msg).map_err(|e| BridgeError::UnexpectedShape(e.to_string()))?;
        let text = LegacyEnvelope::encode(topic, &data)?;
        write_packet(&mut self.sock, text.as_bytes())?;
        Ok(())
    }

    /// Send raw bytes as one packet (adversarial/error-path tests).
    pub fn send_raw(&mut self, payload: &[u8]) -> BridgeResult<()> {
        write_packet(&mut self.sock, payload)?;
        Ok(())
    }

    /// Read one packet, waiting at most `timeout`. `Timeout` and
    /// `Disconnected` are typed, never hangs.
    pub fn recv_packet(&mut self, timeout: Duration) -> BridgeResult<Vec<u8>> {
        self.sock.set_read_timeout(Some(timeout))?;
        let res = read_packet(&mut self.sock);
        let _ = self.sock.set_read_timeout(None);
        res
    }

    /// Read packets until a binary echo sample arrives or `deadline`
    /// passes; `error` replies encountered on the way are collected and
    /// returned with the sample for assertion.
    pub fn recv_echo(
        &mut self,
        deadline: Duration,
        errors: &mut Vec<(String, String)>,
    ) -> BridgeResult<(String, EchoMsg, u32)> {
        let start = std::time::Instant::now();
        loop {
            let rest = deadline.saturating_sub(start.elapsed());
            if rest.is_zero() {
                return Err(BridgeError::Timeout);
            }
            let step = rest.min(Duration::from_millis(200));
            let payload = self.recv_packet(step)?;
            if let Some((code, detail)) = parse_error_reply(&payload) {
                errors.push((code, detail));
                continue;
            }
            let frame = DataFrame::decode(&payload).map_err(BridgeError::Proto)?;
            if frame.flags != FLAG_CDR_LE {
                return Err(BridgeError::UnexpectedShape(format!(
                    "expected CDR frame, flags {:#06x}",
                    frame.flags
                )));
            }
            let msg = decode_echo_xcdr1(&frame.cdr).map_err(BridgeError::Proto)?;
            return Ok((frame.topic, msg, frame.seq));
        }
    }

    /// Like [`recv_echo`](Self::recv_echo) but cooperative: returns
    /// `BridgeError::Cancelled` at the next 200 ms quantum after
    /// `cancel.cancel()`, or `BridgeError::Timeout` on `deadline`.
    pub fn recv_echo_cancel(
        &mut self,
        deadline: Duration,
        errors: &mut Vec<(String, String)>,
        cancel: &CancelFlag,
    ) -> BridgeResult<(String, EchoMsg, u32)> {
        let start = std::time::Instant::now();
        loop {
            if cancel.is_cancelled() {
                return Err(BridgeError::Cancelled);
            }
            let rest = deadline.saturating_sub(start.elapsed());
            if rest.is_zero() {
                return Err(BridgeError::Timeout);
            }
            let step = rest.min(Duration::from_millis(200));
            let payload = match self.recv_packet(step) {
                Ok(p) => p,
                // A quiet socket surfaces as Timeout per quantum: re-check
                // cancellation instead of failing the whole wait.
                Err(BridgeError::Timeout) => continue,
                Err(e) => return Err(e),
            };
            if let Some((code, detail)) = parse_error_reply(&payload) {
                errors.push((code, detail));
                continue;
            }
            let frame = DataFrame::decode(&payload).map_err(BridgeError::Proto)?;
            if frame.flags != FLAG_CDR_LE {
                return Err(BridgeError::UnexpectedShape(format!(
                    "expected CDR frame, flags {:#06x}",
                    frame.flags
                )));
            }
            let msg = decode_echo_xcdr1(&frame.cdr).map_err(BridgeError::Proto)?;
            return Ok((frame.topic, msg, frame.seq));
        }
    }

    /// Send one versioned control message on this connection.
    pub fn send_control(&mut self, ctrl: &Control) -> BridgeResult<()> {
        let json = ctrl.to_json().map_err(BridgeError::Proto)?;
        write_packet(&mut self.sock, json.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> WasmEcho {
        WasmEcho {
            id: 1,
            text: "t".into(),
            values: DdsSequence::from_vec(vec![1, 2]).unwrap(),
        }
    }

    fn stats() -> Arc<StatsInner> {
        Arc::new(StatsInner::default())
    }

    #[test]
    fn oversize_topic_name_fails_fast_without_entering_dds() {
        // Given a topic name no DDS discovery can carry: 8 MiB of text
        // segfaults native `dds_create_topic` (observed SIGSEGV, exit 139)
        // once the participant exists, so pin loopback to reach it.
        // When binding with it, then the gateway must fail fast with a
        // typed error instead of entering FFI.
        let lo = if cfg!(target_os = "macos") {
            "lo0"
        } else {
            "lo"
        };
        std::env::set_var(
            "CYCLONEDDS_URI",
            format!(
                r#"<CycloneDDS><Domain><General><Interfaces><NetworkInterface name="{lo}"/></Interfaces></General></Domain></CycloneDDS>"#
            ),
        );
        let big = format!("T{}", "x".repeat(8 * 1024 * 1024));
        let res = WasmBridge::bind(BridgeConfig {
            topic: big,
            ..BridgeConfig::default()
        });
        assert!(res.is_err(), "oversize topic must fail fast");
        let err = res.err().expect("oversize topic must fail fast");
        assert!(err.to_string().contains("too long"), "unexpected: {err}");
    }

    #[test]
    fn display_names_every_bridge_error() {
        assert!(BridgeError::Io(io::Error::other("x"))
            .to_string()
            .starts_with("io: "));
        assert_eq!(
            BridgeError::Proto(ProtoError::NotConnected).to_string(),
            "proto: not connected"
        );
        assert_eq!(BridgeError::Dds("d".into()).to_string(), "dds: d");
        assert_eq!(BridgeError::Timeout.to_string(), "timed out");
        assert_eq!(BridgeError::Cancelled.to_string(), "cancelled");
        assert_eq!(BridgeError::Disconnected.to_string(), "disconnected");
        assert_eq!(
            BridgeError::UnexpectedShape("s".into()).to_string(),
            "unexpected: s"
        );
    }

    #[test]
    fn from_impls_wrap_io_and_proto() {
        assert!(matches!(
            BridgeError::from(io::Error::other("x")),
            BridgeError::Io(_)
        ));
        assert!(matches!(
            BridgeError::from(ProtoError::NotConnected),
            BridgeError::Proto(_)
        ));
    }

    #[test]
    fn backoff_schedule_doubles_and_caps() {
        let base = Duration::from_millis(10);
        let cap = Duration::from_millis(100);
        assert_eq!(backoff_delay(0, base, cap), base);
        assert_eq!(backoff_delay(1, base, cap), Duration::from_millis(20));
        assert_eq!(backoff_delay(2, base, cap), Duration::from_millis(40));
        assert_eq!(backoff_delay(10, base, cap), cap);
        assert_eq!(backoff_delay(100, base, cap), cap);
    }

    #[test]
    fn cancel_flag_defaults_off_and_latches() {
        let flag = CancelFlag::new();
        assert!(!flag.is_cancelled());
        flag.cancel();
        assert!(flag.is_cancelled());
        assert!(!CancelFlag::default().is_cancelled());
    }

    #[test]
    fn parse_error_reply_accepts_only_error_controls() {
        let err = Control::Error {
            proto: PROTO_VERSION,
            code: "c".into(),
            detail: "d".into(),
        }
        .to_json()
        .unwrap();
        assert_eq!(
            parse_error_reply(err.as_bytes()),
            Some(("c".to_string(), "d".to_string()))
        );
        assert_eq!(parse_error_reply(b"not json"), None);
        let ack = Control::Ack {
            proto: PROTO_VERSION,
            seq: 1,
        }
        .to_json()
        .unwrap();
        assert_eq!(parse_error_reply(ack.as_bytes()), None);
    }

    #[test]
    fn config_lists_primary_first_then_extras() {
        let cfg = BridgeConfig {
            topic: "a".into(),
            extra_topics: vec!["b".into(), "c".into()],
            ..BridgeConfig::default()
        };
        assert_eq!(
            cfg.topics(),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
        assert_eq!(BridgeConfig::default().topics().len(), 1);
    }

    #[test]
    fn await_ready_passes_reports_and_times_out() {
        let (tx, rx) = mpsc::channel();
        tx.send(Ok(())).unwrap();
        await_dds_ready(rx, Duration::from_secs(1)).unwrap();

        let (tx, rx) = mpsc::channel();
        tx.send(Err(BridgeError::Timeout)).unwrap();
        assert!(matches!(
            await_dds_ready(rx, Duration::from_secs(1)),
            Err(BridgeError::Timeout)
        ));

        let (tx, rx) = mpsc::channel::<BridgeResult<()>>();
        drop(tx);
        let err = await_dds_ready(rx, Duration::from_millis(10)).unwrap_err();
        assert!(
            err.to_string().contains("did not report readiness"),
            "{err:?}"
        );
    }

    #[test]
    fn publish_to_dds_counts_or_reports_dead_owner() {
        let st = stats();
        let (tx, rx) = mpsc::channel::<DdsCmd>();
        assert!(publish_to_dds(&tx, "t".into(), sample(), &st));
        let cmd = rx.try_recv().unwrap();
        match cmd {
            DdsCmd::Publish { topic, sample: s } => {
                assert_eq!(topic, "t");
                assert_eq!(s.id, 1);
            }
        }
        assert_eq!(st.snapshot().samples_in, 1);

        let (tx, rx) = mpsc::channel::<DdsCmd>();
        drop(rx);
        assert!(!publish_to_dds(&tx, "t".into(), sample(), &st));
        assert_eq!(st.snapshot().samples_in, 1);
    }

    #[test]
    fn route_prunes_dead_queues_counts_overflow_and_filters_topics() {
        use std::collections::HashSet;
        let stats = stats();
        let registry: Registry = Arc::new(Mutex::new(HashMap::new()));

        // Dead queue (receiver dropped): pruned, no counters move.
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(8);
        drop(rx);
        registry.lock().unwrap().insert(
            1,
            ClientSlot {
                tx,
                subs: Mutex::new(HashSet::from(["t".to_string()])),
            },
        );
        route_to_subscribers(&registry, &stats, "t", &[1, 2, 3]);
        assert!(!registry.lock().unwrap().contains_key(&1));
        assert_eq!(stats.snapshot().dropped_overflow, 0);

        // Full queue (cap 1, pre-filled, never drained): explicit drop.
        let (tx, _rx) = mpsc::sync_channel::<Vec<u8>>(1);
        tx.try_send(vec![9]).unwrap();
        registry.lock().unwrap().insert(
            2,
            ClientSlot {
                tx,
                subs: Mutex::new(HashSet::from(["t".to_string()])),
            },
        );
        route_to_subscribers(&registry, &stats, "t", &[1, 2, 3]);
        assert_eq!(stats.snapshot().dropped_overflow, 1);

        // Unsubscribed topic: no delivery, no counters.
        route_to_subscribers(&registry, &stats, "other", &[1]);
        assert_eq!(stats.snapshot().dropped_overflow, 1);
    }

    #[test]
    fn native_value_conversion_is_lossless() {
        // `DdsNativeValue::to_native_value` has no in-crate caller (it
        // exists for composite `sequence<Struct>` elements); exercising
        // it directly pins the derive's native layout with a real
        // assertion instead of leaving generated code uncovered.
        use cyclonedds::{write_arena::WriteArena, DdsNativeValue, DdsType};
        let msg = sample();
        let mut arena = WriteArena::new();
        let native = msg.to_native_value(&mut arena).unwrap();
        assert_eq!(native.id, 1);
        assert_eq!(native.text.to_string_lossy(), "t");
        assert_eq!(native.values.as_slice(), &[1, 2]);
        // The pointer form used by every `DataWriter::write`: non-null and
        // aliasing the same native layout.
        let ptr = msg.write_to_native(&mut arena).unwrap();
        assert!(!ptr.is_null());
        // SAFETY: ptr is non-null (asserted) and aliases the arena-owned native
        // value of the same layout, alive for this scope.
        let viewed = unsafe { &*(ptr as *const <WasmEcho as DdsType>::Native) };
        assert_eq!(viewed.id, 1);
        assert_eq!(viewed.text.to_string_lossy(), "t");
    }

    #[test]
    fn echo_value_semantics_hold() {
        // Exercises the derived `Debug`/`Clone`/`PartialEq` on WasmEcho.
        let a = sample();
        let b = sample();
        assert_eq!(a, b);
        assert_eq!(a.clone(), b);
        let dbg = format!("{a:?}");
        assert!(dbg.contains("WasmEcho"), "{dbg}");
        let mut c = sample();
        c.id = 2;
        assert_ne!(a, c);
    }

    #[test]
    fn error_packet_is_a_parseable_error_control() {
        let pkt = error_packet("code-x", "detail-y");
        let len = u32::from_le_bytes(pkt[..4].try_into().unwrap()) as usize;
        assert_eq!(len, pkt.len() - 4);
        assert_eq!(
            parse_error_reply(&pkt[4..]),
            Some(("code-x".to_string(), "detail-y".to_string()))
        );
    }
}
