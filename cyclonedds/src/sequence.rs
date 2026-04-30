use crate::{DdsError, DdsResult, DdsString};
use cyclonedds_sys::{dds_alloc, dds_free};
use std::ffi::c_void;
use std::fmt;
use std::ptr;
use std::slice;

pub trait DdsSequenceElement: Clone + Sized + 'static {
    fn sequence_typecode() -> u32;
}

#[repr(C)]
pub struct DdsSequence<T> {
    pub(crate) _maximum: u32,
    pub(crate) _length: u32,
    pub(crate) _buffer: *mut T,
    pub(crate) _release: bool,
}

impl<T> DdsSequence<T> {
    pub const fn new() -> Self {
        Self {
            _maximum: 0,
            _length: 0,
            _buffer: ptr::null_mut(),
            _release: false,
        }
    }

    pub fn len(&self) -> usize {
        self._length as usize
    }

    pub fn is_empty(&self) -> bool {
        self._length == 0
    }

    pub fn as_slice(&self) -> &[T] {
        if self._buffer.is_null() || self._length == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self._buffer.cast_const(), self.len()) }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_slice().iter()
    }
}

impl<T: Clone> DdsSequence<T> {
    pub fn from_slice(values: &[T]) -> DdsResult<Self> {
        if values.is_empty() {
            return Ok(Self::new());
        }

        let elem_size = std::mem::size_of::<T>();
        let total_size = values
            .len()
            .checked_mul(elem_size)
            .ok_or(DdsError::OutOfMemory)?;
        let buffer = unsafe { dds_alloc(total_size) as *mut T };
        if buffer.is_null() {
            return Err(DdsError::OutOfMemory);
        }

        for (idx, value) in values.iter().enumerate() {
            unsafe { ptr::write(buffer.add(idx), value.clone()) };
        }

        Ok(Self {
            _maximum: values.len() as u32,
            _length: values.len() as u32,
            _buffer: buffer,
            _release: true,
        })
    }

    /// # Safety
    ///
    /// `raw` must point to a valid CycloneDDS in-memory sequence instance whose
    /// buffer and length fields describe readable elements of type `T`.
    pub unsafe fn clone_from_raw(raw: *const Self) -> Self {
        let raw = &*raw;
        Self::from_slice(raw.as_slice()).expect("failed to clone DDS sequence")
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

impl<T> Default for DdsSequence<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone> Clone for DdsSequence<T> {
    fn clone(&self) -> Self {
        Self::from_slice(self.as_slice()).expect("failed to clone DDS sequence")
    }
}

impl<T: PartialEq> PartialEq for DdsSequence<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq> Eq for DdsSequence<T> {}

impl<T: fmt::Debug> fmt::Debug for DdsSequence<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DdsSequence")
            .field(&self.as_slice())
            .finish()
    }
}

impl<T> Drop for DdsSequence<T> {
    fn drop(&mut self) {
        if !self._release || self._buffer.is_null() {
            return;
        }

        unsafe {
            if std::mem::needs_drop::<T>() {
                for idx in 0..self.len() {
                    ptr::drop_in_place(self._buffer.add(idx));
                }
            }
            dds_free(self._buffer as *mut c_void);
        }
    }
}

unsafe impl<T: Send> Send for DdsSequence<T> {}
unsafe impl<T: Sync> Sync for DdsSequence<T> {}

#[repr(C)]
pub struct DdsBoundedSequence<T, const N: usize> {
    pub(crate) _maximum: u32,
    pub(crate) _length: u32,
    pub(crate) _buffer: *mut T,
    pub(crate) _release: bool,
}

impl<T, const N: usize> DdsBoundedSequence<T, N> {
    pub const fn new() -> Self {
        Self {
            _maximum: N as u32,
            _length: 0,
            _buffer: ptr::null_mut(),
            _release: false,
        }
    }

    pub fn len(&self) -> usize {
        self._length as usize
    }

    pub fn is_empty(&self) -> bool {
        self._length == 0
    }

    pub fn as_slice(&self) -> &[T] {
        if self._buffer.is_null() || self._length == 0 {
            &[]
        } else {
            unsafe { slice::from_raw_parts(self._buffer.cast_const(), self.len()) }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_slice().iter()
    }
}

impl<T: Clone, const N: usize> DdsBoundedSequence<T, N> {
    pub fn from_slice(values: &[T]) -> DdsResult<Self> {
        if values.len() > N {
            return Err(DdsError::BadParameter(format!(
                "bounded DDS sequence capacity exceeded: len={} bound={}",
                values.len(),
                N
            )));
        }

        if values.is_empty() {
            return Ok(Self::new());
        }

        let elem_size = std::mem::size_of::<T>();
        let total_size = N.checked_mul(elem_size).ok_or(DdsError::OutOfMemory)?;
        let buffer = unsafe { dds_alloc(total_size) as *mut T };
        if buffer.is_null() {
            return Err(DdsError::OutOfMemory);
        }

        unsafe {
            std::ptr::write_bytes(buffer.cast::<u8>(), 0, total_size);
        }

        for (idx, value) in values.iter().enumerate() {
            unsafe { ptr::write(buffer.add(idx), value.clone()) };
        }

        Ok(Self {
            _maximum: N as u32,
            _length: values.len() as u32,
            _buffer: buffer,
            _release: true,
        })
    }

    /// # Safety
    ///
    /// `raw` must point to a valid CycloneDDS in-memory bounded sequence
    /// instance whose buffer and length fields describe readable elements of
    /// type `T`.
    pub unsafe fn clone_from_raw(raw: *const Self) -> Self {
        let raw = &*raw;
        Self::from_slice(raw.as_slice()).expect("failed to clone bounded DDS sequence")
    }

    pub fn to_vec(&self) -> Vec<T> {
        self.as_slice().to_vec()
    }
}

impl<T, const N: usize> Default for DdsBoundedSequence<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Clone, const N: usize> Clone for DdsBoundedSequence<T, N> {
    fn clone(&self) -> Self {
        Self::from_slice(self.as_slice()).expect("failed to clone bounded DDS sequence")
    }
}

impl<T: PartialEq, const N: usize> PartialEq for DdsBoundedSequence<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<T: Eq, const N: usize> Eq for DdsBoundedSequence<T, N> {}

impl<T: fmt::Debug, const N: usize> fmt::Debug for DdsBoundedSequence<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("DdsBoundedSequence")
            .field(&self.as_slice())
            .finish()
    }
}

impl<T, const N: usize> Drop for DdsBoundedSequence<T, N> {
    fn drop(&mut self) {
        if !self._release || self._buffer.is_null() {
            return;
        }

        unsafe {
            if std::mem::needs_drop::<T>() {
                for idx in 0..self.len() {
                    ptr::drop_in_place(self._buffer.add(idx));
                }
            }
            dds_free(self._buffer as *mut c_void);
        }
    }
}

unsafe impl<T: Send, const N: usize> Send for DdsBoundedSequence<T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for DdsBoundedSequence<T, N> {}

macro_rules! impl_primitive_sequence_element {
    ($ty:ty, $typecode:expr) => {
        impl DdsSequenceElement for $ty {
            fn sequence_typecode() -> u32 {
                $typecode
            }
        }
    };
}

impl_primitive_sequence_element!(
    i8,
    crate::TYPE_SEQ | crate::SUBTYPE_1BY | crate::OP_FLAG_SGN
);
impl_primitive_sequence_element!(u8, crate::TYPE_SEQ | crate::SUBTYPE_1BY);
impl_primitive_sequence_element!(
    i16,
    crate::TYPE_SEQ | crate::SUBTYPE_2BY | crate::OP_FLAG_SGN
);
impl_primitive_sequence_element!(u16, crate::TYPE_SEQ | crate::SUBTYPE_2BY);
impl_primitive_sequence_element!(
    i32,
    crate::TYPE_SEQ | crate::SUBTYPE_4BY | crate::OP_FLAG_SGN
);
impl_primitive_sequence_element!(u32, crate::TYPE_SEQ | crate::SUBTYPE_4BY);
impl_primitive_sequence_element!(
    i64,
    crate::TYPE_SEQ | crate::SUBTYPE_8BY | crate::OP_FLAG_SGN
);
impl_primitive_sequence_element!(u64, crate::TYPE_SEQ | crate::SUBTYPE_8BY);
impl_primitive_sequence_element!(
    f32,
    crate::TYPE_SEQ | crate::SUBTYPE_4BY | crate::OP_FLAG_FP
);
impl_primitive_sequence_element!(
    f64,
    crate::TYPE_SEQ | crate::SUBTYPE_8BY | crate::OP_FLAG_FP
);
impl_primitive_sequence_element!(bool, crate::TYPE_SEQ | crate::SUBTYPE_1BY);
impl_primitive_sequence_element!(DdsString, crate::TYPE_SEQ | crate::SUBTYPE_STR);
