//! Content-filtered topics and writer-side topic filters.
//!
//! CycloneDDS implements content filtering through topic-level filter callbacks
//! rather than SQL expression strings. This module provides:
//!
//! - [`ContentFilteredTopic`] – a topic clone with a writer-side filter attached
//!   so only matching samples go on the wire.
//! - Topic-level filter methods on [`Topic`](crate::Topic) (via the
//!   [`TopicFilterExt`] trait) for setting/clearing writer-side filters.

use crate::{
    entity::{DdsEntity, OwnedHandle},
    error::{check, check_entity},
    DdsError, DdsResult, DdsType, Topic,
};
use cyclonedds_rust_sys::*;
use std::collections::HashMap;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::topic::{OP_KOF, OP_RTS};

// ---------------------------------------------------------------------------
// Opaque arg wrapper: keeps the closure + TypeId at a stable heap address
// ---------------------------------------------------------------------------

/// Wrapper stored on the heap; the raw pointer to this is the C `arg`.
struct FilterArg<T> {
    type_id: std::any::TypeId,
    filter: Box<dyn Fn(&T) -> bool + Send + Sync>,
}

// ---------------------------------------------------------------------------
// Trampoline for sample+arg filter closure
// ---------------------------------------------------------------------------

unsafe extern "C" fn trampoline_filter_sample_arg<T: DdsType + 'static>(
    sample: *const c_void,
    arg: *mut c_void,
) -> bool {
    if arg.is_null() || sample.is_null() {
        return true; // pass through when we cannot filter
    }
    let fa: &FilterArg<T> = &*(arg as *const FilterArg<T>);
    // Verify the TypeId matches to catch misuse early.
    if fa.type_id != std::any::TypeId::of::<T>() {
        return true;
    }

    // Panic barrier. CycloneDDS calls this from its own thread, and a panic
    // crossing the `extern "C"` frame aborts the process. Two independent
    // sources of panic reach this point:
    //
    //   * the user's filter closure, and
    //   * `T::clone_out` itself — for a `#[derive(DdsUnionDerive)]` type
    //     declared without a `#[dds_default]` variant it panics on a
    //     discriminator outside the known set, and that discriminator arrives
    //     from the network. Without this barrier a remote peer (or one built
    //     from a different revision of the IDL) could abort this process at
    //     will.
    //
    // Fail closed on panic (exclude the sample) rather than passing unfiltered
    // data through, matching `waitset.rs::trampoline_qc_filter`.
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // The sample pointer comes from write_to_native, which may point to a
        // #[repr(C)] native struct rather than the user struct T.  clone_out
        // correctly interprets the native-layout pointer, so use it instead of
        // a raw cast to *const T.
        // An undecodable sample cannot be filtered, so exclude it -- same
        // fail-closed choice the panic path below makes.
        match T::clone_out(sample as *const T) {
            Ok(data) => (fa.filter)(&data),
            Err(_) => false,
        }
    })) {
        Ok(keep) => keep,
        Err(_) => {
            eprintln!("cyclonedds: content filter panicked; sample excluded");
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Descriptor keepalive (mirrors topic.rs DescriptorHolder)
// ---------------------------------------------------------------------------

struct CftDescriptorHolder {
    _ops: Vec<u32>,
    _typename: std::ffi::CString,
    _key_names: Vec<std::ffi::CString>,
    _keys: Vec<dds_key_descriptor>,
    _meta: std::ffi::CString,
}

// ---------------------------------------------------------------------------
// ContentFilteredTopic
// ---------------------------------------------------------------------------

/// A **content-filtered topic** wraps a [`Topic`] with a writer-side filter
/// closure.  Only samples for which the closure returns `true` are written to
/// the network.
///
/// Internally this creates a sibling DDS topic entity (sharing the same type
/// descriptor) and attaches a filter callback via
/// `dds_set_topic_filter_extended`.
///
/// # Lifetime
///
/// The `ContentFilteredTopic` keeps the original `Topic`'s descriptor data
/// alive through an `Rc` reference.  The filter closure is heap-allocated and
/// freed when the CFT is dropped or the filter is replaced.
pub struct ContentFilteredTopic<T: DdsType> {
    entity: dds_entity_t,
    // The Box<FilterArg<T>> stays at a stable address; the C API holds a
    // pointer into it.  Replacing this field invalidates the old pointer,
    // but we always call dds_set_topic_filter_extended before dropping.
    _filter_arg: Option<Box<FilterArg<T>>>,
    _desc_holder: Rc<CftDescriptorHolder>,
    _marker: PhantomData<T>,
    /// The topic this was filtered from, and through it the participant.
    ///
    /// Declared last so it is released only after `Drop` has cleared the C
    /// filter and deleted the entity. Like `QueryCondition`, this type keeps its
    /// own `Drop` instead of moving into `OwnedEntity`, because the filter has
    /// to be detached before the entity goes.
    _parents: Vec<std::sync::Arc<crate::entity::OwnedEntity>>,
}

impl<T: DdsType + 'static> ContentFilteredTopic<T> {
    /// Create a new content-filtered topic from an existing [`Topic`].
    ///
    /// The `filter` closure is called for every sample written through any
    /// writer created from this topic.  Return `true` to allow the sample,
    /// `false` to silently drop it.
    pub fn new<F>(topic: &Topic<T>, filter: F) -> DdsResult<Self>
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        // Get the parent participant so we can create a sibling topic entity.
        let participant = unsafe { dds_get_participant(topic.entity()) };
        check_entity(participant)?;

        let (handle, desc_holder) = Self::create_sibling_topic(participant, topic.entity())?;

        // Build the filter arg on the heap.
        let filter_arg: Box<FilterArg<T>> = Box::new(FilterArg {
            type_id: std::any::TypeId::of::<T>(),
            filter: Box::new(filter),
        });
        let arg_ptr = &*filter_arg as *const FilterArg<T> as *mut c_void;

        unsafe {
            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_SAMPLE_ARG;
            dds_filter.f.sample_arg = Some(trampoline_filter_sample_arg::<T>);
            dds_filter.arg = arg_ptr;

            let ret = dds_set_topic_filter_extended(handle, &dds_filter as *const dds_topic_filter);
            if ret < 0 {
                dds_delete(handle);
                return Err(DdsError::from(ret));
            }
        }

        Ok(ContentFilteredTopic {
            entity: handle,
            _filter_arg: Some(filter_arg),
            _desc_holder: desc_holder,
            _marker: PhantomData,
            _parents: vec![topic.owned().clone()],
        })
    }

    /// Replace the filter closure at runtime.
    ///
    /// The new closure replaces the old one.  The DDS filter is updated
    /// atomically before the old closure is dropped.
    /// # Safety
    ///
    /// CycloneDDS requires that no reader or writer uses this topic while its
    /// filter is replaced. The caller must provide that external exclusion.
    pub unsafe fn set_filter<F>(&mut self, filter: F) -> DdsResult<()>
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
    {
        let filter_arg: Box<FilterArg<T>> = Box::new(FilterArg {
            type_id: std::any::TypeId::of::<T>(),
            filter: Box::new(filter),
        });
        let arg_ptr = &*filter_arg as *const FilterArg<T> as *mut c_void;

        unsafe {
            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_SAMPLE_ARG;
            dds_filter.f.sample_arg = Some(trampoline_filter_sample_arg::<T>);
            dds_filter.arg = arg_ptr;

            let ret = dds_set_topic_filter_extended(
                self.entity(),
                &dds_filter as *const dds_topic_filter,
            );
            check(ret)?;
        }

        // Only drop the old arg *after* the DDS filter has been updated.
        self._filter_arg = Some(filter_arg);
        Ok(())
    }

    /// Remove the filter, allowing all samples through.
    ///
    /// # Safety
    ///
    /// No reader or writer may use this topic while the filter is cleared.
    pub unsafe fn clear_filter(&mut self) -> DdsResult<()> {
        unsafe {
            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_NONE;

            let ret = dds_set_topic_filter_extended(
                self.entity(),
                &dds_filter as *const dds_topic_filter,
            );
            check(ret)?;
        }

        // Safe to drop the arg now that the C side no longer references it.
        self._filter_arg = None;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Create a sibling topic entity with the same type descriptor.
    fn create_sibling_topic(
        participant: dds_entity_t,
        original: dds_entity_t,
    ) -> DdsResult<(dds_entity_t, Rc<CftDescriptorHolder>)> {
        unsafe {
            let type_name = std::ffi::CString::new(T::type_name())
                .map_err(|_| DdsError::BadParameter("type name contains null".into()))?;

            let topic_name_c = {
                let mut buf = vec![0u8; 256];
                let n = dds_get_name(original, buf.as_mut_ptr() as *mut i8, buf.len());
                if n < 0 {
                    return Err(DdsError::from(n));
                }
                buf.truncate(n as usize);
                let original_name = String::from_utf8_lossy(&buf);
                let clone_name = format!("{}_cft_{}", original_name, original);
                std::ffi::CString::new(clone_name)
                    .map_err(|_| DdsError::BadParameter("topic name contains null".into()))?
            };

            let mut ops = T::ops();
            if ops.last().copied() != Some(OP_RTS) {
                ops.push(OP_RTS);
            }

            let key_defs = T::keys();
            let key_names: Vec<std::ffi::CString> = key_defs
                .iter()
                .map(|k| {
                    std::ffi::CString::new(k.name.as_str())
                        .map_err(|_| DdsError::BadParameter("key name contains null".into()))
                })
                .collect::<DdsResult<Vec<_>>>()?
                .into_iter()
                .collect();
            let mut keys: Vec<dds_key_descriptor> = Vec::with_capacity(key_defs.len());
            for (i, kd) in key_defs.iter().enumerate() {
                let offset = ops.len() as u32;
                ops.push(OP_KOF | (kd.ops_path.len() as u32));
                ops.extend(kd.ops_path.iter().copied());
                keys.push(dds_key_descriptor {
                    m_name: key_names[i].as_ptr(),
                    m_offset: offset,
                    m_idx: i as u32,
                });
            }

            let post_key_ops = T::post_key_ops();
            if !post_key_ops.is_empty() {
                ops.extend(post_key_ops);
            }
            let meta = std::ffi::CString::default();

            let descriptor = dds_topic_descriptor {
                m_size: T::descriptor_size(),
                m_align: T::descriptor_align(),
                m_flagset: T::flagset(),
                m_nkeys: T::key_count() as u32,
                m_typename: type_name.as_ptr(),
                m_keys: if keys.is_empty() {
                    std::ptr::null()
                } else {
                    keys.as_ptr()
                },
                m_nops: ops.len() as u32,
                m_ops: ops.as_ptr(),
                m_meta: meta.as_ptr(),
                type_information: std::mem::zeroed(),
                type_mapping: std::mem::zeroed(),
                restrict_data_representation: 0,
            };

            let handle = dds_create_topic(
                participant,
                &descriptor,
                topic_name_c.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
            );
            check_entity(handle)?;

            // Wrap the descriptor data in an Rc so it outlives the topic.
            let holder = Rc::new(CftDescriptorHolder {
                _ops: ops,
                _typename: type_name,
                _key_names: key_names,
                _keys: keys,
                _meta: meta,
            });

            Ok((handle, holder))
        }
    }
}

impl<T: DdsType> DdsEntity for ContentFilteredTopic<T> {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl<T: DdsType> Drop for ContentFilteredTopic<T> {
    fn drop(&mut self) {
        // Clear the filter first so the C side releases the arg pointer
        // before we drop it.
        unsafe {
            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_NONE;
            dds_set_topic_filter_extended(self.entity, &dds_filter as *const dds_topic_filter);
        }
        crate::entity::delete_entity(self.entity, "ContentFilteredTopic");
    }
}

// ---------------------------------------------------------------------------
// TopicFilterExt – convenience methods on Topic<T>
// ---------------------------------------------------------------------------

/// Extension trait that adds writer-side filter support to [`Topic<T>`].
///
/// Unlike [`ContentFilteredTopic`] (which creates a separate topic entity),
/// these methods set the filter directly on the existing topic, affecting
/// all writers created from it.
pub trait TopicFilterExt<T: DdsType + 'static> {
    /// Set a writer-side filter that drops samples before they go on the wire.
    ///
    /// The filter closure receives a reference to the sample and returns
    /// `true` if the sample should be sent, `false` to drop it silently.
    ///
    /// Only one filter can be active at a time; calling this replaces any
    /// previously set filter.
    ///
    /// ```compile_fail
    /// use cyclonedds::{DdsTypeDerive, DomainParticipant, Topic, TopicFilterExt};
    ///
    /// #[derive(DdsTypeDerive)]
    /// struct Message { value: i32 }
    ///
    /// let participant = DomainParticipant::new(0).unwrap();
    /// let topic = Topic::<Message>::new(&participant, "filtered").unwrap();
    /// topic.set_filter(|sample| sample.value > 0).unwrap();
    /// ```
    ///
    /// # Safety
    ///
    /// CycloneDDS requires external exclusion: no reader or writer may use
    /// this topic while its filter is replaced.
    unsafe fn set_filter<F: Fn(&T) -> bool + Send + Sync + 'static>(
        &self,
        filter: F,
    ) -> DdsResult<()>;

    /// Remove any previously set writer-side filter.
    ///
    /// # Safety
    ///
    /// No reader or writer may use this topic while the filter is cleared.
    unsafe fn clear_filter(&self) -> DdsResult<()>;
}

impl<T: DdsType + 'static> TopicFilterExt<T> for Topic<T> {
    unsafe fn set_filter<F: Fn(&T) -> bool + Send + Sync + 'static>(
        &self,
        filter: F,
    ) -> DdsResult<()> {
        let filter_arg: Box<FilterArg<T>> = Box::new(FilterArg {
            type_id: std::any::TypeId::of::<T>(),
            filter: Box::new(filter),
        });
        let arg_ptr = &*filter_arg as *const FilterArg<T> as *mut c_void;

        unsafe {
            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_SAMPLE_ARG;
            dds_filter.f.sample_arg = Some(trampoline_filter_sample_arg::<T>);
            dds_filter.arg = arg_ptr;

            let ret = dds_set_topic_filter_extended(
                self.entity(),
                &dds_filter as *const dds_topic_filter,
            );
            if ret < 0 {
                return Err(DdsError::from(ret));
            }
        }
        self.owned().replace_callback_state(Some(filter_arg));
        Ok(())
    }

    unsafe fn clear_filter(&self) -> DdsResult<()> {
        unsafe {
            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_NONE;
            let ret = dds_set_topic_filter_extended(
                self.entity(),
                &dds_filter as *const dds_topic_filter,
            );
            check(ret)?;
        }
        self.owned().replace_callback_state(None);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Parameterized filters (dynamic parameter updates at runtime)
// ---------------------------------------------------------------------------

/// Shared parameter store for content filters.
///
/// `FilterParams` holds a map of named integer parameters that can be updated
/// at runtime without recreating the filter closure.
///
/// # Example
/// ```no_run
/// use cyclonedds::{ContentFilteredTopic, FilterParams};
///
/// let params = FilterParams::new();
/// params.set("min_id", 10);
/// params.set("max_id", 100);
/// ```
pub struct FilterParams {
    inner: Arc<Mutex<HashMap<String, i64>>>,
}

impl FilterParams {
    pub fn new() -> Self {
        FilterParams {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set a parameter value.
    pub fn set(&self, key: impl Into<String>, value: i64) {
        let mut p = self.inner.lock().unwrap();
        p.insert(key.into(), value);
    }

    /// Get a parameter value.
    pub fn get(&self, key: &str) -> Option<i64> {
        let p = self.inner.lock().unwrap();
        p.get(key).copied()
    }

    /// Remove a parameter.
    pub fn remove(&self, key: &str) {
        let mut p = self.inner.lock().unwrap();
        p.remove(key);
    }

    pub(crate) fn clone_inner(&self) -> Arc<Mutex<HashMap<String, i64>>> {
        self.inner.clone()
    }
}

impl Default for FilterParams {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension trait for creating parameterized content filters on [`Topic`].
pub trait TopicParameterizedFilterExt<T: DdsType + 'static> {
    /// Create a content-filtered topic with a parameterized filter.
    ///
    /// The `filter` closure receives a reference to the sample and the current
    /// parameter store. Parameters can be updated at any time via the returned
    /// [`FilterParams`] without recreating the topic.
    fn with_params<F>(&self, filter: F) -> DdsResult<(ContentFilteredTopic<T>, FilterParams)>
    where
        F: Fn(&T, &FilterParams) -> bool + Send + Sync + 'static;
}

impl<T: DdsType + 'static> TopicParameterizedFilterExt<T> for Topic<T> {
    fn with_params<F>(&self, filter: F) -> DdsResult<(ContentFilteredTopic<T>, FilterParams)>
    where
        F: Fn(&T, &FilterParams) -> bool + Send + Sync + 'static,
    {
        let params = FilterParams::new();
        let params_clone = params.clone_inner();

        let cft = ContentFilteredTopic::new(self, move |sample| {
            let p = FilterParams {
                inner: params_clone.clone(),
            };
            filter(sample, &p)
        })?;

        Ok((cft, params))
    }
}
