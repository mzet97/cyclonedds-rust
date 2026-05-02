//! DDS error types and helpers.

#[cfg(not(feature = "no_std"))]
use thiserror::Error;

#[cfg(not(feature = "no_std"))]
#[derive(Debug, Error)]
pub enum DdsError {
    #[error("DDS error code: {0}")]
    ReturnCode(i32),

    #[error("invalid entity handle: {0}")]
    InvalidEntity(i32),

    #[error("out of resources")]
    OutOfResources,

    #[error("operation timed out")]
    Timeout,

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("bad parameter: {0}")]
    BadParameter(String),

    #[error("precondition not met: {0}")]
    PreconditionNotMet(String),

    #[error("out of memory")]
    OutOfMemory,

    #[error("already deleted")]
    AlreadyDeleted,

    #[error("DDS: {0}")]
    Other(String),
}

#[cfg(feature = "no_std")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DdsError {
    ReturnCode(i32),
    InvalidEntity(i32),
    OutOfResources,
    Timeout,
    Unsupported(alloc::string::String),
    BadParameter(alloc::string::String),
    PreconditionNotMet(alloc::string::String),
    OutOfMemory,
    AlreadyDeleted,
    Other(alloc::string::String),
}

#[cfg(feature = "no_std")]
impl core::fmt::Display for DdsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DdsError::ReturnCode(c) => write!(f, "DDS error code: {c}"),
            DdsError::InvalidEntity(c) => write!(f, "invalid entity handle: {c}"),
            DdsError::OutOfResources => write!(f, "out of resources"),
            DdsError::Timeout => write!(f, "operation timed out"),
            DdsError::Unsupported(s) => write!(f, "unsupported: {s}"),
            DdsError::BadParameter(s) => write!(f, "bad parameter: {s}"),
            DdsError::PreconditionNotMet(s) => write!(f, "precondition not met: {s}"),
            DdsError::OutOfMemory => write!(f, "out of memory"),
            DdsError::AlreadyDeleted => write!(f, "already deleted"),
            DdsError::Other(s) => write!(f, "DDS: {s}"),
        }
    }
}

#[cfg(feature = "no_std")]
impl core::error::Error for DdsError {}

impl From<i32> for DdsError {
    fn from(code: i32) -> Self {
        match code {
            0 => DdsError::Other("unexpected success-to-error conversion".into()),
            -1 => DdsError::ReturnCode(code),
            -2 => DdsError::OutOfMemory,
            -3 => DdsError::BadParameter("bad parameter".into()),
            -4 => DdsError::PreconditionNotMet("precondition not met".into()),
            -5 => DdsError::OutOfResources,
            -6 => DdsError::Other("not enabled".into()),
            -7 => DdsError::Unsupported("immutable policy".into()),
            -8 => DdsError::Unsupported("inconsistent policy".into()),
            -9 => DdsError::AlreadyDeleted,
            -10 => DdsError::Timeout,
            -11 => DdsError::Other("no data".into()),
            -12 => DdsError::Unsupported("unsupported".into()),
            _ => DdsError::ReturnCode(code),
        }
    }
}

pub type DdsResult<T> = Result<T, DdsError>;

impl DdsError {
    pub fn raw_code(&self) -> Option<i32> {
        match self {
            DdsError::ReturnCode(code) | DdsError::InvalidEntity(code) => Some(*code),
            DdsError::OutOfMemory => Some(-2),
            DdsError::BadParameter(_) => Some(-3),
            DdsError::PreconditionNotMet(_) => Some(-4),
            DdsError::OutOfResources => Some(-5),
            DdsError::AlreadyDeleted => Some(-9),
            DdsError::Timeout => Some(-10),
            DdsError::Unsupported(_) | DdsError::Other(_) => None,
        }
    }

    /// Returns `true` if this error is likely transient and the operation
    /// may succeed on retry (e.g. timeout, temporary resource exhaustion).
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            DdsError::Timeout
                | DdsError::OutOfResources
                | DdsError::OutOfMemory
                | DdsError::ReturnCode(_)
        )
    }
}

#[cfg(not(feature = "no_std"))]
pub fn err_nr(code: i32) -> i32 {
    cyclonedds_rust_sys::dds_err_nr(code)
}

#[cfg(not(feature = "no_std"))]
pub fn err_line(code: i32) -> u32 {
    cyclonedds_rust_sys::dds_err_line(code)
}

#[cfg(not(feature = "no_std"))]
pub fn err_file_id(code: i32) -> u32 {
    cyclonedds_rust_sys::dds_err_file_id(code)
}

#[cfg(not(feature = "no_std"))]
pub fn check(ret: i32) -> DdsResult<()> {
    if ret >= 0 {
        Ok(())
    } else {
        Err(DdsError::from(ret))
    }
}

#[cfg(not(feature = "no_std"))]
pub fn check_entity(ret: i32) -> DdsResult<i32> {
    if ret >= 0 {
        Ok(ret)
    } else {
        Err(DdsError::from(ret))
    }
}

#[cfg(not(feature = "no_std"))]
#[cfg(test)]
mod tests {
    use super::{err_file_id, err_line, err_nr, DdsError};

    #[test]
    fn public_error_helpers_match_current_c_header_macros() {
        assert_eq!(err_nr(-10), -10);
        assert_eq!(err_line(-10), 0);
        assert_eq!(err_file_id(-10), 0);
    }

    #[test]
    fn raw_code_exposes_structured_return_codes() {
        assert_eq!(DdsError::from(-10).raw_code(), Some(-10));
        assert_eq!(DdsError::from(-3).raw_code(), Some(-3));
        assert_eq!(DdsError::Unsupported("x".into()).raw_code(), None);
    }
}
