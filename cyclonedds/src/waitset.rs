use crate::error::check_entity;
use crate::{DdsEntity, DdsResult};
use cyclonedds_sys::*;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// Global registry for QueryCondition closures
// ---------------------------------------------------------------------------
// The C API `dds_create_querycondition` accepts a filter function with
// signature `fn(*const c_void) -> bool` – no context/arg parameter.  To
// bridge Rust closures we store them in a global map keyed by the condition
// entity handle.  The trampoline dispatches through this map.

fn qc_registry() -> &'static Mutex<HashMap<dds_entity_t, Box<dyn Fn(*const c_void) -> bool + Send + Sync>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<dds_entity_t, Box<dyn Fn(*const c_void) -> bool + Send + Sync>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Trampoline used by all closure-backed QueryConditions.
///
/// Since the C callback doesn't pass an arg, we use a thread-local
/// "active condition" key that is set before each read/take/wait operation.
unsafe extern "C" fn trampoline_qc_filter(sample: *const c_void) -> bool {
    QC_ACTIVE.with(|active| {
        let handle = active.get();
        if handle <= 0 {
            return true; // no active condition, pass through
        }
        let registry = qc_registry().lock().unwrap();
        if let Some(closure) = registry.get(&handle) {
            closure(sample)
        } else {
            true // condition not found, pass through
        }
    })
}

thread_local! {
    static QC_ACTIVE: std::cell::Cell<dds_entity_t> = std::cell::Cell::new(0);
}

/// Set the active QueryCondition entity for the current thread.
///
/// Call this before DDS read/take/wait operations that may invoke a
/// QueryCondition filter.  Reset to 0 after the operation.
pub fn set_active_qc(entity: dds_entity_t) {
    QC_ACTIVE.with(|active| active.set(entity));
}

// ---------------------------------------------------------------------------
// WaitSet
// ---------------------------------------------------------------------------

pub struct WaitSet {
    entity: dds_entity_t,
}

impl WaitSet {
    pub fn new(participant: dds_entity_t) -> DdsResult<Self> {
        let entity = unsafe { dds_create_waitset(participant) };
        check_entity(entity)?;
        Ok(WaitSet { entity })
    }

    pub fn attach(&self, entity: dds_entity_t, cookie: i64) -> DdsResult<()> {
        let ret = unsafe { dds_waitset_attach(self.entity, entity, cookie as dds_attach_t) };
        crate::error::check(ret)
    }

    pub fn detach(&self, entity: dds_entity_t) -> DdsResult<()> {
        let ret = unsafe { dds_waitset_detach(self.entity, entity) };
        crate::error::check(ret)
    }

    pub fn set_trigger(&self, trigger: bool) -> DdsResult<()> {
        let ret = unsafe { dds_waitset_set_trigger(self.entity, trigger) };
        crate::error::check(ret)
    }

    pub fn wait(&self, timeout_ns: i64) -> DdsResult<Vec<i64>> {
        let max_results: usize = 64;
        let mut xs: Vec<dds_attach_t> = vec![0; max_results];
        let n = unsafe { dds_waitset_wait(self.entity, xs.as_mut_ptr(), max_results, timeout_ns) };
        if n < 0 {
            return Err(crate::DdsError::from(n));
        }
        let n = n as usize;
        xs.truncate(n);
        Ok(xs.into_iter().map(|x| x as i64).collect())
    }

    pub fn get_entities(&self) -> DdsResult<Vec<dds_entity_t>> {
        unsafe {
            let count = dds_waitset_get_entities(self.entity, std::ptr::null_mut(), 0);
            if count < 0 {
                return Err(crate::DdsError::from(count));
            }
            let count = count as usize;
            if count == 0 {
                return Ok(Vec::new());
            }

            let mut entities = vec![0; count];
            let actual =
                dds_waitset_get_entities(self.entity, entities.as_mut_ptr(), entities.len());
            if actual < 0 {
                return Err(crate::DdsError::from(actual));
            }
            entities.truncate(actual as usize);
            Ok(entities)
        }
    }
}

impl DdsEntity for WaitSet {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl Drop for WaitSet {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}

// ---------------------------------------------------------------------------
// ReadCondition
// ---------------------------------------------------------------------------

pub struct ReadCondition {
    entity: dds_entity_t,
}

impl ReadCondition {
    pub fn new(reader: dds_entity_t, mask: u32) -> DdsResult<Self> {
        let entity = unsafe { dds_create_readcondition(reader, mask) };
        check_entity(entity)?;
        Ok(ReadCondition { entity })
    }

    pub fn any(reader: dds_entity_t) -> DdsResult<Self> {
        let mask = cyclonedds_sys::DDS_ANY_SAMPLE_STATE
            | cyclonedds_sys::DDS_ANY_INSTANCE_STATE
            | cyclonedds_sys::DDS_ANY_VIEW_STATE;
        Self::new(reader, mask)
    }

    pub fn not_read(reader: dds_entity_t) -> DdsResult<Self> {
        let mask = cyclonedds_sys::DDS_NOT_READ_SAMPLE_STATE
            | cyclonedds_sys::DDS_ANY_INSTANCE_STATE
            | cyclonedds_sys::DDS_ANY_VIEW_STATE;
        Self::new(reader, mask)
    }
}

impl DdsEntity for ReadCondition {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl Drop for ReadCondition {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}

// ---------------------------------------------------------------------------
// QueryCondition
// ---------------------------------------------------------------------------

pub struct QueryCondition {
    entity: dds_entity_t,
    /// Whether this QueryCondition owns a closure in the global registry.
    owns_closure: bool,
}

impl QueryCondition {
    /// Create a QueryCondition with a raw C function pointer filter.
    ///
    /// This is the low-level constructor matching the C API directly.
    /// For a Rust-closure-based API, see [`QueryCondition::with_filter`].
    pub fn new(
        reader: dds_entity_t,
        mask: u32,
        filter: unsafe extern "C" fn(*const std::ffi::c_void) -> bool,
    ) -> DdsResult<Self> {
        let entity = unsafe { dds_create_querycondition(reader, mask, Some(filter)) };
        check_entity(entity)?;
        Ok(QueryCondition {
            entity,
            owns_closure: false,
        })
    }

    /// Create a QueryCondition with a Rust closure as the filter.
    ///
    /// The closure receives a raw `*const c_void` pointer to the sample data
    /// (matching the C API contract).  Return `true` to include the sample,
    /// `false` to exclude it.
    ///
    /// # Note on thread-local activation
    ///
    /// Because the CycloneDDS C API does not pass a user-data argument to
    /// the filter callback, the closure is dispatched through a thread-local
    /// "active handle" mechanism.  Before any DDS operation that may trigger
    /// this filter (e.g., `dds_read`, `dds_take`, `dds_waitset_wait`), call
    /// [`set_active_qc`] with this condition's entity handle.  Reset it to
    /// 0 afterward.
    pub fn with_filter<F>(reader: dds_entity_t, mask: u32, filter: F) -> DdsResult<Self>
    where
        F: Fn(*const c_void) -> bool + Send + Sync + 'static,
    {
        let entity = unsafe {
            dds_create_querycondition(reader, mask, Some(trampoline_qc_filter))
        };
        check_entity(entity)?;

        // Register the closure.
        {
            let mut registry = qc_registry().lock().unwrap();
            registry.insert(entity, Box::new(filter));
        }

        Ok(QueryCondition {
            entity,
            owns_closure: true,
        })
    }
}

impl DdsEntity for QueryCondition {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl Drop for QueryCondition {
    fn drop(&mut self) {
        if self.owns_closure {
            let mut registry = qc_registry().lock().unwrap();
            registry.remove(&self.entity);
        }
        unsafe {
            dds_delete(self.entity);
        }
    }
}

// ---------------------------------------------------------------------------
// GuardCondition
// ---------------------------------------------------------------------------

pub struct GuardCondition {
    entity: dds_entity_t,
}

impl GuardCondition {
    pub fn new(participant: dds_entity_t) -> DdsResult<Self> {
        let entity = unsafe { dds_create_guardcondition(participant) };
        check_entity(entity)?;
        Ok(GuardCondition { entity })
    }

    pub fn set_triggered(&self, triggered: bool) -> DdsResult<()> {
        let ret = unsafe { dds_set_guardcondition(self.entity, triggered) };
        crate::error::check(ret)
    }

    /// Read the current trigger state without consuming it.
    pub fn read(&self) -> DdsResult<bool> {
        let mut triggered = false;
        let ret = unsafe { dds_read_guardcondition(self.entity, &mut triggered) };
        crate::error::check(ret)?;
        Ok(triggered)
    }

    /// Take (read and reset) the current trigger state.
    pub fn take(&self) -> DdsResult<bool> {
        let mut triggered = false;
        let ret = unsafe { dds_take_guardcondition(self.entity, &mut triggered) };
        crate::error::check(ret)?;
        Ok(triggered)
    }
}

impl DdsEntity for GuardCondition {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl Drop for GuardCondition {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
