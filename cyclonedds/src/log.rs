//! DDS Logging API – configurable log and trace sinks.
//!
//! Provides Rust callbacks for CycloneDDS log and trace output.  The C API
//! uses function-pointer + void-pointer pairs; this module hides that behind
//! `Box<dyn Fn(LogEntry)>` closures stored in process-global slots.

use cyclonedds_sys::*;
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
type LogCallback = Box<dyn Fn(LogEntry) + Send + Sync>;

/// Global slot for the *log* sink callback.
static LOG_SINK: Mutex<Option<LogCallback>> = Mutex::new(None);

/// Global slot for the *trace* sink callback.
static TRACE_SINK: Mutex<Option<LogCallback>> = Mutex::new(None);

// ---------------------------------------------------------------------------
// C trampoline
// ---------------------------------------------------------------------------

/// Shared trampoline function passed to `dds_set_log_sink` /
/// `dds_set_trace_sink`.
///
/// The `logdatum` pointer is a transparent `Box<LogCallback>` stored in the
/// corresponding global.  We reconstruct a reference to it and call the
/// Rust closure with a [`LogEntry`] built from the C data.
unsafe extern "C" fn log_trampoline(_logdatum: *mut std::ffi::c_void, data: *const dds_log_data_t) {
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
        let s = bytes
            .split(|&b| b == 0)
            .next()
            .unwrap_or(&[]);
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

    // _logdatum is unused – we look up the global instead.  This is safe
    // because the trampoline is only active while the global contains a
    // callback.
    // We cannot know which sink (log vs trace) fired from here, so we
    // attempt both.  Only the one that is set will invoke.
    if let Ok(guard) = LOG_SINK.lock() {
        if let Some(ref cb) = *guard {
            cb(entry.clone());
        }
    }
    if let Ok(guard) = TRACE_SINK.lock() {
        if let Some(ref cb) = *guard {
            cb(entry);
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Install (or remove) the **log** sink callback.
///
/// Pass `None` to restore the default sink (writes to stderr).
pub fn set_log_sink(callback: Option<Box<dyn Fn(LogEntry) + Send + Sync>>) {
    let mut slot = LOG_SINK.lock().unwrap();
    *slot = callback;
    let (cb, arg) = if slot.is_some() {
        (
            Some(log_trampoline as unsafe extern "C" fn(*mut std::ffi::c_void, *const cyclonedds_sys::dds_log_data_t)),
            std::ptr::null_mut::<std::ffi::c_void>(),
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
    let mut slot = TRACE_SINK.lock().unwrap();
    *slot = callback;
    let (cb, arg) = if slot.is_some() {
        (
            Some(log_trampoline as unsafe extern "C" fn(*mut std::ffi::c_void, *const cyclonedds_sys::dds_log_data_t)),
            std::ptr::null_mut::<std::ffi::c_void>(),
        )
    } else {
        (None, std::ptr::null_mut::<std::ffi::c_void>())
    };
    unsafe {
        dds_set_trace_sink(cb, arg);
    }
}
