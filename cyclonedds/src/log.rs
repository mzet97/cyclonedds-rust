//! DDS Logging API – configurable log and trace sinks.
//!
//! Provides Rust callbacks for CycloneDDS log and trace output.  The C API
//! uses function-pointer + void-pointer pairs; this module hides that behind
//! `Box<dyn Fn(LogEntry)>` closures stored in process-global slots.

use cyclonedds_rust_sys::*;
use std::ffi::CStr;
use std::sync::Mutex;

// ---------------------------------------------------------------------------
// Log category
// ---------------------------------------------------------------------------

/// A bit-mask of log category / priority levels.
///
/// Maps directly to the `DDS_LOG_*` / `DDS_TRACE_*` constants used by
/// CycloneDDS internally (the `dds_log_data_t.priority` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogCategory(pub u32);

impl LogCategory {
    /// No categories.
    pub const NONE: LogCategory = LogCategory(0);
    /// Fatal errors.
    pub const FATAL: LogCategory = LogCategory(1);
    /// Non-fatal errors.
    pub const ERROR: LogCategory = LogCategory(2);
    /// Warnings.
    pub const WARNING: LogCategory = LogCategory(4);
    /// Informational messages.
    pub const INFO: LogCategory = LogCategory(8);
    /// Debug-level messages.
    pub const DEBUG: LogCategory = LogCategory(16);
    /// Fine-grained trace messages.
    pub const TRACE: LogCategory = LogCategory(32);
    /// All categories combined.
    pub const ALL: LogCategory = LogCategory(0x3F);
}

impl std::ops::BitOr for LogCategory {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        LogCategory(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for LogCategory {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

// ---------------------------------------------------------------------------
// Log entry – what the Rust callback receives
// ---------------------------------------------------------------------------

/// A single log message delivered to a Rust callback.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// The formatted log message.
    pub message: String,
    /// Category / priority of the message.
    pub category: LogCategory,
    /// Source file name (if available).
    pub file: Option<String>,
    /// Source line number (if available).
    pub line: Option<u32>,
    /// Source function name (if available).
    pub function: Option<String>,
    /// DDS domain id (`None` for global messages).
    pub domain_id: Option<u32>,
}

// ---------------------------------------------------------------------------
// Global callback storage
// ---------------------------------------------------------------------------

/// Type-erased callback that the C trampoline will invoke.
///
/// `Arc`, not `Box`, so the trampoline can clone it out and release the mutex
/// *before* running user code. Invoking the callback while holding the lock
/// deadlocks the process if that callback logs anything that flows back through
/// CycloneDDS.
type LogCallback = std::sync::Arc<dyn Fn(LogEntry) + Send + Sync>;

/// Global slot for the *log* sink callback.
static LOG_SINK: Mutex<Option<LogCallback>> = Mutex::new(None);

/// Global slot for the *trace* sink callback.
static TRACE_SINK: Mutex<Option<LogCallback>> = Mutex::new(None);

// Sink discriminators. Both sinks share one trampoline, so without a tag it is
// impossible to tell which one fired — the previous version passed a null
// `logdatum` for both and simply invoked *both* sinks, delivering every line
// twice whenever a log sink and a trace sink were installed together.
//
// The addresses of these statics are the tags (an integer-to-pointer cast is
// not permitted in const context).
static LOG_SINK_TAG: u8 = 0;
static TRACE_SINK_TAG: u8 = 0;

fn log_tag() -> *mut std::ffi::c_void {
    &LOG_SINK_TAG as *const u8 as *mut std::ffi::c_void
}

fn trace_tag() -> *mut std::ffi::c_void {
    &TRACE_SINK_TAG as *const u8 as *mut std::ffi::c_void
}

// ---------------------------------------------------------------------------
// C trampoline
// ---------------------------------------------------------------------------

/// Shared trampoline function passed to `dds_set_log_sink` /
/// `dds_set_trace_sink`.
///
/// The `logdatum` pointer is a transparent `Box<LogCallback>` stored in the
/// corresponding global.  We reconstruct a reference to it and call the
/// Rust closure with a [`LogEntry`] built from the C data.
unsafe extern "C" fn log_trampoline(logdatum: *mut std::ffi::c_void, data: *const dds_log_data_t) {
    if data.is_null() {
        return;
    }
    let d = &*data;

    // Build the message.  d.message is a C string of length d.size.
    let message = if d.message.is_null() || d.size == 0 {
        String::new()
    } else {
        // size includes the trailing NUL in some CycloneDDS builds, but
        // to_string_lossy handles that gracefully.
        let bytes = std::slice::from_raw_parts(d.message as *const u8, d.size);
        let s = bytes.split(|&b| b == 0).next().unwrap_or(&[]);
        String::from_utf8_lossy(s).into_owned()
    };

    let file = if d.file.is_null() {
        None
    } else {
        Some(CStr::from_ptr(d.file).to_string_lossy().into_owned())
    };

    let function = if d.function.is_null() {
        None
    } else {
        Some(CStr::from_ptr(d.function).to_string_lossy().into_owned())
    };

    // domain id: UINT32_MAX (u32::MAX) means "global"
    let domain_id = if d.domid == u32::MAX {
        None
    } else {
        Some(d.domid)
    };

    let entry = LogEntry {
        message,
        category: LogCategory(d.priority),
        file,
        line: if d.line == 0 { None } else { Some(d.line) },
        function,
        domain_id,
    };

    // `logdatum` tells us which sink fired, so each line is delivered exactly
    // once. A null tag means the sink was registered by an older code path;
    // fall back to trying both rather than dropping the line.
    let slots: &[&Mutex<Option<LogCallback>>] = if logdatum == log_tag() {
        &[&LOG_SINK]
    } else if logdatum == trace_tag() {
        &[&TRACE_SINK]
    } else {
        &[&LOG_SINK, &TRACE_SINK]
    };

    for slot in slots {
        // Clone the Arc out and drop the guard before calling: holding the
        // mutex across user code deadlocks if that code logs.
        let callback = match slot.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        };
        let Some(cb) = callback else { continue };

        // Panic barrier: this is an `extern "C"` frame called from a CycloneDDS
        // thread, so a panicking user sink would abort the process. Contain it
        // and keep going — losing a log line beats losing the application.
        let entry = entry.clone();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || cb(entry)));
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Install (or remove) the **log** sink callback.
///
/// Pass `None` to restore the default sink (writes to stderr).
pub fn set_log_sink(callback: Option<Box<dyn Fn(LogEntry) + Send + Sync>>) {
    let mut slot = LOG_SINK.lock().unwrap_or_else(|e| e.into_inner());
    *slot = callback.map(LogCallback::from);
    let (cb, arg) = if slot.is_some() {
        (
            Some(
                log_trampoline
                    as unsafe extern "C" fn(
                        *mut std::ffi::c_void,
                        *const cyclonedds_rust_sys::dds_log_data_t,
                    ),
            ),
            log_tag(),
        )
    } else {
        (None, std::ptr::null_mut::<std::ffi::c_void>())
    };
    unsafe {
        dds_set_log_sink(cb, arg);
    }
}

/// Install (or remove) the **trace** sink callback.
///
/// Pass `None` to restore the default sink (writes to stderr).
pub fn set_trace_sink(callback: Option<Box<dyn Fn(LogEntry) + Send + Sync>>) {
    let mut slot = TRACE_SINK.lock().unwrap_or_else(|e| e.into_inner());
    *slot = callback.map(LogCallback::from);
    let (cb, arg) = if slot.is_some() {
        (
            Some(
                log_trampoline
                    as unsafe extern "C" fn(
                        *mut std::ffi::c_void,
                        *const cyclonedds_rust_sys::dds_log_data_t,
                    ),
            ),
            trace_tag(),
        )
    } else {
        (None, std::ptr::null_mut::<std::ffi::c_void>())
    };
    unsafe {
        dds_set_trace_sink(cb, arg);
    }
}
