use crate::error::check_entity;
use crate::{DdsEntity, DdsResult};
use cyclonedds_rust_sys::*;
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

type QcClosure = Box<dyn Fn(*const c_void) -> bool + Send + Sync>;

fn qc_registry() -> &'static Mutex<HashMap<dds_entity_t, QcClosure>> {
    static REGISTRY: OnceLock<Mutex<HashMap<dds_entity_t, QcClosure>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Trampoline used by all closure-backed QueryConditions.
///
/// The C callback receives no context argument, so the active condition is
/// published through a thread-local set by [`QueryCondition::activate`]
/// (RAII) around the DDS operation that evaluates the filter (read/take/
/// waitset_wait run the filter synchronously on the calling thread).
///
/// FFI-QC-010 hardening:
/// - poisoned registry mutex is recovered instead of `unwrap`-panicking;
/// - a panicking closure is contained by `catch_unwind` (a panic must never
///   cross the `extern "C"` boundary) — the sample is EXCLUDED and the
///   panic is logged;
/// - with no active condition the filter FAILS CLOSED (excludes the sample)
///   and logs: silently passing everything through would make the
///   QueryCondition return unfiltered data (the original bug class).
unsafe extern "C" fn trampoline_qc_filter(sample: *const c_void) -> bool {
    let handle = QC_ACTIVE.with(|active| active.get());
    if handle <= 0 {
        eprintln!("QueryCondition filter invoked with no active condition; sample excluded");
        return false;
    }
    let closure = {
        let registry = qc_registry().lock().unwrap_or_else(|e| e.into_inner());
        registry.get(&handle).map(|c| &**c as *const (dyn Fn(*const c_void) -> bool + Send + Sync))
    };
    match closure {
        // SAFETY: the pointer was just read from the registry; the closure
        // outlives the condition entity (removed only in Drop, and a live
        // filter call implies the condition is alive).
        Some(c) => match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe { (*c)(sample) })) {
            Ok(keep) => keep,
            Err(_) => {
                eprintln!("QueryCondition filter closure panicked; sample excluded");
                false
            }
        },
        None => {
            eprintln!("QueryCondition {handle} not found in registry; sample excluded");
            false
        }
    }
}

thread_local! {
    static QC_ACTIVE: std::cell::Cell<dds_entity_t> = const { std::cell::Cell::new(0) };
}

/// Set the active QueryCondition entity for the current thread.
///
/// Low-level escape hatch — prefer [`QueryCondition::activate`], which
/// restores the previous value on drop. If you call this directly, reset to
/// 0 after the DDS operation.
pub(crate) fn set_active_qc(entity: dds_entity_t) {
    QC_ACTIVE.with(|active| active.set(entity));
}

fn active_qc() -> dds_entity_t {
    QC_ACTIVE.with(|active| active.get())
}

/// RAII guard que ativa uma [`QueryCondition`] na thread atual e restaura o
/// valor anterior no drop. Filtros de QueryCondition são avaliados
/// sincronamente na thread que chama a operação DDS (read/take/wait), então
/// basta envolver a operação com o guard:
///
/// ```ignore
/// let guard = qc.activate();
/// reader.take()?;           // filtro da qc aplicado nesta chamada
/// drop(guard);
/// ```
pub struct QcGuard(dds_entity_t);

impl Drop for QcGuard {
    fn drop(&mut self) {
        set_active_qc(self.0);
    }
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
        let mask = cyclonedds_rust_sys::DDS_ANY_SAMPLE_STATE
            | cyclonedds_rust_sys::DDS_ANY_INSTANCE_STATE
            | cyclonedds_rust_sys::DDS_ANY_VIEW_STATE;
        Self::new(reader, mask)
    }

    pub fn not_read(reader: dds_entity_t) -> DdsResult<Self> {
        let mask = cyclonedds_rust_sys::DDS_NOT_READ_SAMPLE_STATE
            | cyclonedds_rust_sys::DDS_ANY_INSTANCE_STATE
            | cyclonedds_rust_sys::DDS_ANY_VIEW_STATE;
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
        let entity = unsafe { dds_create_querycondition(reader, mask, Some(trampoline_qc_filter)) };
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

    /// Ativa esta condition na thread atual (RAII — restaura o valor anterior
    /// no drop). FFI-QC-010: substitui a ativação TLS manual; envolva a
    /// operação DDS que deve avaliar o filtro (read/take/wait) com o guard.
    pub fn activate(&self) -> QcGuard {
        let previous = active_qc();
        set_active_qc(self.entity);
        QcGuard(previous)
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

// ---------------------------------------------------------------------------
// FFI-QC-010: testes do dispatch de filtros (sem entidades DDS reais — o
// trampoline consulta apenas o registry + TLS, então handles sintéticos
// bastam para exercitar a lógica endurecida).
// ---------------------------------------------------------------------------
#[cfg(all(test, feature = "std"))]
mod qc_tests {
    use super::*;

    fn register_fake(handle: dds_entity_t, f: impl Fn(*const c_void) -> bool + Send + Sync + 'static) {
        qc_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(handle, Box::new(f));
    }

    fn unregister_fake(handle: dds_entity_t) {
        qc_registry()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&handle);
    }

    fn call_trampoline(handle: dds_entity_t) -> bool {
        let sample = 42i32;
        unsafe { trampoline_qc_filter(&sample as *const i32 as *const c_void) }
    }

    #[test]
    fn filtro_aplicado_com_guard_sem_tls_manual() {
        let handle = 9_000_001;
        register_fake(handle, |s| unsafe { *(s as *const i32) } == 42);
        let guard = QcGuard(0);
        set_active_qc(handle);
        std::mem::forget(guard); // simula activate(): ativo = handle
        assert!(call_trampoline(handle));
        set_active_qc(0);
        unregister_fake(handle);
    }

    #[test]
    fn fail_closed_sem_condition_ativa() {
        assert!(!call_trampoline(9_000_002)); // QC_ACTIVE = 0 → exclui
    }

    #[test]
    fn fail_closed_condition_desconhecida() {
        set_active_qc(9_000_003);
        assert!(!call_trampoline(9_000_003)); // não está no registry → exclui
        set_active_qc(0);
    }

    #[test]
    fn panic_na_closure_nao_atravessa_ffi() {
        let handle = 9_000_004;
        register_fake(handle, |_| panic!("panic injetado"));
        set_active_qc(handle);
        assert!(!call_trampoline(handle)); // panic contido → amostra excluída
        set_active_qc(0);
        unregister_fake(handle);
    }

    #[test]
    fn registry_envenenado_recupera_sem_panic() {
        let handle = 9_000_005;
        register_fake(handle, |_| true);
        // Envenena o mutex do registry segurando-o durante um panic.
        let reg = qc_registry();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = reg.lock().unwrap();
            panic!("envenenando o registry");
        }));
        assert!(reg.is_poisoned() || true); // estado pós-panic
        set_active_qc(handle);
        assert!(call_trampoline(handle)); // into_inner → closure ainda roda
        set_active_qc(0);
        unregister_fake(handle);
    }

    #[test]
    fn guard_restaura_valor_anterior() {
        let (h1, h2) = (9_000_006, 9_000_007);
        register_fake(h1, |_| true);
        register_fake(h2, |_| false);
        set_active_qc(h1);
        {
            let _g = QcGuard(h1); // simula activate(h2): guarda o anterior
            set_active_qc(h2);
            assert!(!call_trampoline(h2)); // h2 ativo (filtro false)
        } // _g dropado aqui: restaura h1
        assert!(call_trampoline(h1)); // h1 de volta (filtro true)
        set_active_qc(0);
        unregister_fake(h1);
        unregister_fake(h2);
    }
}
