//! Fault injection in a dedicated process: file-descriptor exhaustion
//! makes the pump-socket `try_clone` fail, so `serve_conn` must drop the
//! unserved connection (client observes EOF) instead of half-serving it —
//! and the gateway must keep serving everyone else.
//!
//! This binary exists because stuffing the process-wide fd table would
//! disturb every test sharing a process; here the only sockets are the
//! gateway's own, pinned to loopback.

//! Unix-only: `RLIMIT_NOFILE` + `/dev/null` stuffing do not exist on Windows.
#![cfg(unix)]

use dds_wasm_bridge::{BridgeConfig, WasmBridge};
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Loopback interface name is OS-specific (`lo0` on macOS).
#[cfg(target_os = "macos")]
const LO_IF: &str = "lo0";
#[cfg(not(target_os = "macos"))]
const LO_IF: &str = "lo";

fn lo_uri() -> String {
    format!(
        r#"<CycloneDDS><Domain><General><Interfaces><NetworkInterface name="{LO_IF}"/></Interfaces></General></Domain></CycloneDDS>"#
    )
}

/// Lowers `RLIMIT_NOFILE` and stuffs every remaining slot with
/// `/dev/null`. Restores the limit and releases the stuffing on drop, so
/// an assertion failure cannot leak a crippled fd table.
struct FdStuffGuard {
    stuffed: Vec<File>,
    saved: libc::rlimit,
}

impl FdStuffGuard {
    fn engage() -> Result<Self, String> {
        let mut saved = std::mem::MaybeUninit::<libc::rlimit>::uninit();
        // SAFETY: RLIMIT_NOFILE is valid; as_mut_ptr of a live MaybeUninit is writable.
        if unsafe { libc::getrlimit(libc::RLIMIT_NOFILE, saved.as_mut_ptr()) } != 0 {
            return Err("getrlimit(RLIMIT_NOFILE) failed".into());
        }
        // SAFETY: getrlimit returned 0, so the struct was initialized.
        let saved = unsafe { saved.assume_init() };
        let low = libc::rlimit {
            rlim_cur: 256,
            rlim_max: saved.rlim_max,
        };
        // SAFETY: rlim_cur (256) <= rlim_max just read; lowering the soft limit is allowed.
        if unsafe { libc::setrlimit(libc::RLIMIT_NOFILE, &low) } != 0 {
            return Err("setrlimit(RLIMIT_NOFILE) failed".into());
        }
        let mut stuffed = Vec::new();
        while stuffed.len() < 100_000 {
            match File::open("/dev/null") {
                Ok(f) => stuffed.push(f),
                Err(_) => break, // EMFILE: table exhausted.
            }
        }
        if stuffed.len() < 2 {
            // SAFETY: restoring the saved limit; &saved borrows a live local.
            unsafe {
                libc::setrlimit(libc::RLIMIT_NOFILE, &saved);
            }
            return Err("fd table would not exhaust".into());
        }
        Ok(FdStuffGuard { stuffed, saved })
    }

    fn free_one(&mut self) {
        self.stuffed.pop();
    }
}

impl Drop for FdStuffGuard {
    fn drop(&mut self) {
        self.stuffed.clear();
        // SAFETY: restoring the saved limit; stuffed fds were closed first.
        unsafe {
            libc::setrlimit(libc::RLIMIT_NOFILE, &self.saved);
        }
    }
}

fn send_packet(sock: &mut TcpStream, payload: &[u8]) {
    sock.write_all(&(payload.len() as u32).to_le_bytes())
        .unwrap();
    sock.write_all(payload).unwrap();
    sock.flush().unwrap();
}

fn recv_packet(sock: &mut TcpStream, timeout: Duration) -> Option<Vec<u8>> {
    sock.set_read_timeout(Some(timeout)).ok()?;
    let mut len = [0u8; 4];
    sock.read_exact(&mut len).ok()?;
    let n = u32::from_le_bytes(len) as usize;
    if n > 64 * 1024 * 1024 {
        return None;
    }
    let mut payload = vec![0u8; n];
    sock.read_exact(&mut payload).ok()?;
    Some(payload)
}

fn unique_topic(tag: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("FdEx_{tag}_{}_{nanos}", std::process::id())
}

fn attempt(tag: &str) -> Result<(), String> {
    // Given a live gateway plus one healthy witness connection.
    std::env::set_var("CYCLONEDDS_URI", lo_uri());
    let bridge = WasmBridge::bind(BridgeConfig {
        topic: unique_topic(tag),
        ..BridgeConfig::default()
    })
    .map_err(|e| format!("bind: {e}"))?;
    eprintln!("fd_probe: bound {tag} at {}", bridge.addr());
    let mut witness = TcpStream::connect(bridge.addr()).map_err(|e| format!("witness: {e}"))?;
    eprintln!("fd_probe: {tag} witness connected");
    send_packet(
        &mut witness,
        br#"{"proto":0,"kind":"hello","client":"fd-witness-pre"}"#,
    );
    recv_packet(&mut witness, Duration::from_secs(5)).ok_or("witness not served")?;
    eprintln!("fd_probe: {tag} witness served (pre-hello acked)");

    // When the fd table is exhausted, a new client still completes its
    // TCP handshake (kernel backlog) but the server cannot clone a pump
    // socket for it.
    let mut guard = FdStuffGuard::engage()?;
    eprintln!("fd_probe: {tag} table stuffed");
    guard.free_one(); // exactly one slot: the victim's own socket.
    let mut victim =
        TcpStream::connect(bridge.addr()).map_err(|e| format!("victim connect: {e}"))?;
    std::thread::sleep(Duration::from_millis(100)); // accept keeps failing.
    guard.free_one(); // the next accept poll succeeds; the clones fail.

    // Then the unserved connection is dropped (EOF) instead of hanging.
    victim
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("timeout: {e}"))?;
    let mut buf = [0u8; 1];
    match victim.read(&mut buf) {
        Ok(0) => {}
        other => {
            return Err(format!(
                "victim must see EOF, got {:?}",
                std::mem::discriminant(&other)
            ))
        }
    }
    eprintln!("fd_probe: {tag} victim saw EOF");
    drop(guard); // unstuff + restore the limit before the liveness proof.
    eprintln!("fd_probe: {tag} guard dropped");

    // And the gateway still serves: hello on the witness gets an ack. Poll
    // with a deadline: post-exhaustion recovery time (threads re-arming
    // after EMFILE) is platform-dependent; the invariant is eventual
    // service on the SAME pre-exhaustion socket, not instant service.
    let hello = br#"{"proto":0,"kind":"hello","client":"fd-witness"}"#;
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let reply = loop {
        send_packet(&mut witness, hello);
        match recv_packet(&mut witness, Duration::from_secs(2)) {
            Some(reply) => break reply,
            None if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(100));
            }
            None => return Err("witness never recovered after exhaustion".into()),
        }
    };
    if dds_wasm_bridge::parse_error_reply(&reply).is_some() {
        return Err("hello got an error reply".into());
    }
    bridge.close();
    Ok(())
}

#[test]
fn pump_clone_failure_drops_unserved_connection_and_gateway_survives() {
    let mut last = String::from("no attempt ran");
    for i in 0..3 {
        match attempt(&format!("try{i}")) {
            Ok(()) => return,
            Err(e) => {
                last = e;
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    panic!("fd exhaustion never produced the early-return path: {last}");
}
