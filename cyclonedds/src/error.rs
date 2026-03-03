//! Error types for CycloneDDS operations

use thiserror::Error;

/// Errors that can occur during DDS operations
#[derive(Debug, Error)]
pub enum DdsError {
    /// DDS return code indicates error
    #[error("DDS returned error code: {0}")]
    ReturnCode(i32),

    /// Entity is invalid
    #[error("Invalid DDS entity: {0}")]
    InvalidEntity(u32),

    /// Out of resources
    #[error("Out of resources")]
    OutOfResources,

    /// Timeout occurred
    #[error("Operation timed out")]
    Timeout,

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Bad parameter
    #[error("Bad parameter: {0}")]
    BadParameter(String),

    /// Precondition not met
    #[error("Precondition not met: {0}")]
    PreconditionNotMet(String),

    /// Out of memory
    #[error("Out of memory")]
    OutOfMemory,

    /// Other error
    #[error("DDS error: {0}")]
    Other(String),
}

impl From<i32> for DdsError {
    fn from(code: i32) -> Self {
        match code {
            0 => return DdsError::Other("Success (unexpected conversion)".to_string()),
            -1 => DdsError::OutOfResources,
            -2 => DdsError::OutOfMemory,
            -3 => DdsError::InvalidEntity(0),
            -4 => DdsError::BadParameter("Bad parameter".to_string()),
            -5 => DdsError::Timeout,
            -6 => DdsError::PreconditionNotMet("Precondition not met".to_string()),
            -7 => DdsError::Unsupported("Unsupported".to_string()),
            _ => DdsError::ReturnCode(code),
        }
    }
}

impl From<u32> for DdsError {
    fn from(entity: u32) -> Self {
        DdsError::InvalidEntity(entity)
    }
}

/// Result type alias for DDS operations
pub type DdsResult<T> = Result<T, DdsError>;
