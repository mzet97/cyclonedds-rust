use crate::{error::check, DdsError, DdsResult};
use cyclonedds_rust_sys::*;
use std::ffi::{CStr, CString};
use std::slice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticKind {
    Uint32,
    Uint64,
    LengthTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatisticValue {
    Uint32(u32),
    Uint64(u64),
    LengthTime(u64),
}

#[derive(Clone, Copy)]
pub struct StatisticEntryRef<'a> {
    raw: &'a dds_stat_keyvalue,
}

impl<'a> StatisticEntryRef<'a> {
    pub fn name(&self) -> &'a str {
        if self.raw.name.is_null() {
            ""
        } else {
            unsafe { CStr::from_ptr(self.raw.name).to_str().unwrap_or("") }
        }
    }

    pub fn kind(&self) -> StatisticKind {
        match self.raw.kind {
            x if x == dds_stat_kind_DDS_STAT_KIND_UINT32 => StatisticKind::Uint32,
            x if x == dds_stat_kind_DDS_STAT_KIND_UINT64 => StatisticKind::Uint64,
            x if x == dds_stat_kind_DDS_STAT_KIND_LENGTHTIME => StatisticKind::LengthTime,
            _ => StatisticKind::Uint64,
        }
    }

    pub fn value(&self) -> StatisticValue {
        unsafe {
            match self.kind() {
                StatisticKind::Uint32 => StatisticValue::Uint32(self.raw.u.u32_),
                StatisticKind::Uint64 => StatisticValue::Uint64(self.raw.u.u64_),
                StatisticKind::LengthTime => StatisticValue::LengthTime(self.raw.u.lengthtime),
            }
        }
    }
}

pub struct Statistics {
    ptr: *mut dds_statistics,
}

impl Statistics {
    pub(crate) fn new(entity: dds_entity_t) -> DdsResult<Self> {
        let ptr = unsafe { dds_create_statistics(entity) };
        if ptr.is_null() {
            return Err(DdsError::Unsupported(
                "statistics unavailable for this entity or build".into(),
            ));
        }
        Ok(Self { ptr })
    }

    pub fn refresh(&mut self) -> DdsResult<()> {
        unsafe { check(dds_refresh_statistics(self.ptr)) }
    }

    pub fn entity(&self) -> dds_entity_t {
        self.raw().entity
    }

    pub fn timestamp(&self) -> dds_time_t {
        self.raw().time
    }

    pub fn len(&self) -> usize {
        self.raw().count
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn entries(&self) -> impl Iterator<Item = StatisticEntryRef<'_>> {
        self.entries_slice()
            .iter()
            .map(|raw| StatisticEntryRef { raw })
    }

    pub fn lookup(&self, name: &str) -> DdsResult<Option<StatisticEntryRef<'_>>> {
        let c_name = CString::new(name)
            .map_err(|_| DdsError::BadParameter("statistic names cannot contain NUL".into()))?;
        let ptr = unsafe { dds_lookup_statistic(self.ptr.cast_const(), c_name.as_ptr()) };
        if ptr.is_null() {
            Ok(None)
        } else {
            Ok(Some(StatisticEntryRef {
                raw: unsafe { &*ptr },
            }))
        }
    }

    fn raw(&self) -> &dds_statistics {
        unsafe { &*self.ptr }
    }

    fn entries_slice(&self) -> &[dds_stat_keyvalue] {
        unsafe { slice::from_raw_parts(self.raw().kv.as_ptr(), self.raw().count) }
    }
}

impl Drop for Statistics {
    fn drop(&mut self) {
        unsafe { dds_delete_statistics(self.ptr) };
    }
}
