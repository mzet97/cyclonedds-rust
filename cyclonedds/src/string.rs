use crate::{DdsError, DdsResult};
use cyclonedds_rust_sys::{dds_string_dup, dds_string_free};
use std::ffi::{CStr, CString};
use std::fmt;
use std::os::raw::c_char;

#[repr(transparent)]
pub struct DdsString {
    ptr: *mut c_char,
}

impl DdsString {
    pub fn new(value: impl AsRef<str>) -> DdsResult<Self> {
        let c_string = CString::new(value.as_ref()).map_err(|_| {
            DdsError::BadParameter("DDS strings cannot contain interior NUL bytes".into())
        })?;
        let ptr = unsafe { dds_string_dup(c_string.as_ptr()) };
        if ptr.is_null() {
            return Err(DdsError::OutOfMemory);
        }
        Ok(Self { ptr })
    }

    pub const fn null() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
        }
    }

    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    pub fn as_ptr(&self) -> *const c_char {
        self.ptr.cast_const()
    }

    pub fn to_string_lossy(&self) -> String {
        if self.ptr.is_null() {
            return String::new();
        }
        unsafe { CStr::from_ptr(self.ptr).to_string_lossy().into_owned() }
    }
}

impl Default for DdsString {
    fn default() -> Self {
        Self::new("").expect("failed to allocate empty DDS string")
    }
}

impl Clone for DdsString {
    fn clone(&self) -> Self {
        if self.ptr.is_null() {
            return Self::null();
        }
        let ptr = unsafe { dds_string_dup(self.ptr.cast_const()) };
        assert!(
            !ptr.is_null(),
            "failed to clone DDS string: CycloneDDS returned null"
        );
        Self { ptr }
    }
}

impl Drop for DdsString {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { dds_string_free(self.ptr) };
        }
    }
}

impl fmt::Debug for DdsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DdsString")
            .field(&self.to_string_lossy())
            .finish()
    }
}

impl fmt::Display for DdsString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_lossy())
    }
}

impl PartialEq for DdsString {
    fn eq(&self, other: &Self) -> bool {
        self.to_string_lossy() == other.to_string_lossy()
    }
}

impl Eq for DdsString {}

impl TryFrom<&str> for DdsString {
    type Error = DdsError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<String> for DdsString {
    type Error = DdsError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

unsafe impl Send for DdsString {}
unsafe impl Sync for DdsString {}
