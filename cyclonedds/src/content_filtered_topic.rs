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
    entity::DdsEntity,
    error::{check, check_entity},
    DdsError, DdsResult, DdsType, Topic,
};
use cyclonedds_rust_sys::*;
use std::ffi::c_void;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::topic::{OP_RTS, OP_KOF};

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
    // The sample pointer comes from write_to_native, which may point to a
    // #[repr(C)] native struct rather than the user struct T.  clone_out
    // correctly interprets the native-layout pointer, so use it instead of
    // a raw cast to *const T.
    let data = T::clone_out(sample as *const T);
    (fa.filter)(&data)
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

        let (handle, desc_holder) =
            Self::create_sibling_topic(participant, topic.entity())?;

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

            let ret =
                dds_set_topic_filter_extended(handle, &dds_filter as *const dds_topic_filter);
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
        })
    }

    /// Replace the filter closure at runtime.
    ///
    /// The new closure replaces the old one.  The DDS filter is updated
    /// atomically before the old closure is dropped.
    pub fn set_filter<F>(&mut self, filter: F) -> DdsResult<()>
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

            let ret =
                dds_set_topic_filter_extended(self.entity(), &dds_filter as *const dds_topic_filter);
            check(ret)?;
        }

        // Only drop the old arg *after* the DDS filter has been updated.
        self._filter_arg = Some(filter_arg);
        Ok(())
    }

    /// Remove the filter, allowing all samples through.
    pub fn clear_filter(&mut self) -> DdsResult<()> {
        unsafe {
            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_NONE;

            let ret =
                dds_set_topic_filter_extended(self.entity(), &dds_filter as *const dds_topic_filter);
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
                .map(|k| std::ffi::CString::new(k.name.as_str()).unwrap())
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
            let meta = std::ffi::CString::new("").unwrap();

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
            dds_delete(self.entity);
        }
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
    fn set_filter<F: Fn(&T) -> bool + Send + Sync + 'static>(&self, filter: F) -> DdsResult<()>;

    /// Remove any previously set writer-side filter.
    fn clear_filter(&self) -> DdsResult<()>;
}

// We store the filter arg in a thread-safe box that outlives the C callback.
// Since Topic<T> doesn't have a field for this, we use a leaked Box and
// track it.  This is acceptable because Topic is typically long-lived and
// there's usually only one filter per topic.
//
// A more elaborate design could use a HashMap<entity, Box> but for the
// common case this is overkill.  Instead, we use the "set and forget"
// pattern: the arg is leaked on set, and re-leaked on clear.

impl<T: DdsType + 'static> TopicFilterExt<T> for Topic<T> {
    fn set_filter<F: Fn(&T) -> bool + Send + Sync + 'static>(&self, filter: F) -> DdsResult<()> {
        // Step 1: Capture the old arg pointer so we can free it later.
        let old_arg = unsafe {
            let mut old_fn: dds_topic_filter_arg_fn = None;
            let mut old_arg: *mut c_void = std::ptr::null_mut();
            let _ = dds_get_topic_filter_and_arg(self.entity(), &mut old_fn, &mut old_arg);
            old_arg
        };

        // Step 2: Build new filter arg and set it.
        let filter_arg: Box<FilterArg<T>> = Box::new(FilterArg {
            type_id: std::any::TypeId::of::<T>(),
            filter: Box::new(filter),
        });
        let arg_ptr = Box::into_raw(filter_arg) as *mut c_void;

        unsafe {
            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_SAMPLE_ARG;
            dds_filter.f.sample_arg = Some(trampoline_filter_sample_arg::<T>);
            dds_filter.arg = arg_ptr;

            let ret =
                dds_set_topic_filter_extended(self.entity(), &dds_filter as *const dds_topic_filter);
            if ret < 0 {
                // Reconstruct the Box and drop it to free memory.
                let _ = Box::from_raw(arg_ptr as *mut FilterArg<T>);
                return Err(DdsError::from(ret));
            }
        }

        // Step 3: Free the old arg now that the new filter is active.
        if !old_arg.is_null() {
            unsafe {
                let _ = Box::from_raw(old_arg as *mut FilterArg<T>);
            }
        }

        Ok(())
    }

    fn clear_filter(&self) -> DdsResult<()> {
        // Retrieve the old arg so we can free it.
        unsafe {
            let mut old_fn: dds_topic_filter_arg_fn = None;
            let mut old_arg: *mut c_void = std::ptr::null_mut();
            let _ = dds_get_topic_filter_and_arg(
                self.entity(),
                &mut old_fn,
                &mut old_arg,
            );

            let mut dds_filter: dds_topic_filter = std::mem::zeroed();
            dds_filter.mode = dds_topic_filter_mode_DDS_TOPIC_FILTER_NONE;
            let ret =
                dds_set_topic_filter_extended(self.entity(), &dds_filter as *const dds_topic_filter);
            check(ret)?;

            // Free the old arg if we got one.
            if !old_arg.is_null() {
                let _ = Box::from_raw(old_arg as *mut FilterArg<T>);
            }
        }
        Ok(())
    }
}
