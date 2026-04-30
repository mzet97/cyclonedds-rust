use crate::{
    error::check,
    qos::Qos,
    xtypes::{FindScope, TopicDescriptor, TypeInfo, TypeObject},
    DdsError, DdsResult, DdsType, UntypedTopic,
};
use cyclonedds_rust_sys::*;
use std::ffi::{c_char, CStr};

pub const DDS_MIN_PSEUDO_HANDLE: dds_entity_t = 0x7fff0000u32 as dds_entity_t;
pub const BUILTIN_TOPIC_DCPSPARTICIPANT: dds_entity_t = DDS_MIN_PSEUDO_HANDLE + 1;
pub const BUILTIN_TOPIC_DCPSTOPIC: dds_entity_t = DDS_MIN_PSEUDO_HANDLE + 2;
pub const BUILTIN_TOPIC_DCPSPUBLICATION: dds_entity_t = DDS_MIN_PSEUDO_HANDLE + 3;
pub const BUILTIN_TOPIC_DCPSSUBSCRIPTION: dds_entity_t = DDS_MIN_PSEUDO_HANDLE + 4;

fn dup_cstr(ptr: *const c_char) -> *mut c_char {
    if ptr.is_null() {
        std::ptr::null_mut()
    } else {
        unsafe { dds_string_dup(ptr) }
    }
}

#[repr(C)]
pub struct BuiltinParticipantSample {
    key: dds_guid_t,
    qos: *mut dds_qos_t,
}

unsafe impl Send for BuiltinParticipantSample {}

impl BuiltinParticipantSample {
    pub fn key(&self) -> dds_guid_t {
        self.key
    }

    pub fn qos(&self) -> DdsResult<Option<Qos>> {
        Qos::from_raw_clone(self.qos)
    }

    /// Returns the participant name if set (via the QoS entity_name policy).
    ///
    /// In CycloneDDS, the participant name is not a direct field of
    /// `dds_builtintopic_participant_t` but is carried as the entity name
    /// in the participant's QoS.
    pub fn participant_name(&self) -> Option<String> {
        self.qos()
            .ok()
            .flatten()
            .and_then(|q| q.entity_name().ok().flatten())
    }
}

impl Clone for BuiltinParticipantSample {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            qos: Qos::from_raw_clone(self.qos)
                .ok()
                .flatten()
                .map_or(std::ptr::null_mut(), |q| {
                    let ptr = q.as_ptr() as *mut dds_qos_t;
                    std::mem::forget(q);
                    ptr
                }),
        }
    }
}

impl Drop for BuiltinParticipantSample {
    fn drop(&mut self) {
        if !self.qos.is_null() {
            unsafe { dds_delete_qos(self.qos) };
        }
    }
}

impl DdsType for BuiltinParticipantSample {
    fn type_name() -> &'static str {
        "DCPSParticipant"
    }

    fn ops() -> Vec<u32> {
        Vec::new()
    }

    unsafe fn clone_out(ptr: *const Self) -> Self {
        let src = &*ptr;
        src.clone()
    }
}

#[repr(C)]
pub struct BuiltinTopicSample {
    key: dds_builtintopic_topic_key_t,
    topic_name: *mut c_char,
    type_name: *mut c_char,
    qos: *mut dds_qos_t,
}

unsafe impl Send for BuiltinTopicSample {}

impl BuiltinTopicSample {
    pub fn key(&self) -> dds_builtintopic_topic_key_t {
        self.key
    }

    pub fn topic_name(&self) -> String {
        if self.topic_name.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(self.topic_name) }
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn type_name_value(&self) -> String {
        if self.type_name.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(self.type_name) }
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn qos(&self) -> DdsResult<Option<Qos>> {
        Qos::from_raw_clone(self.qos)
    }

    pub fn find_topic(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<UntypedTopic>> {
        let name = std::ffi::CString::new(self.topic_name())
            .map_err(|_| DdsError::BadParameter("topic name contains null".into()))?;
        let handle = unsafe {
            dds_find_topic(
                scope.as_raw(),
                participant,
                name.as_ptr(),
                std::ptr::null(),
                timeout,
            )
        };
        if handle == 0 {
            return Ok(None);
        }
        crate::error::check_entity(handle).map(|entity| Some(UntypedTopic::from_entity(entity)))
    }
}

impl Clone for BuiltinTopicSample {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            topic_name: dup_cstr(self.topic_name),
            type_name: dup_cstr(self.type_name),
            qos: Qos::from_raw_clone(self.qos)
                .ok()
                .flatten()
                .map_or(std::ptr::null_mut(), |q| {
                    let ptr = q.as_ptr() as *mut dds_qos_t;
                    std::mem::forget(q);
                    ptr
                }),
        }
    }
}

impl Drop for BuiltinTopicSample {
    fn drop(&mut self) {
        if !self.topic_name.is_null() {
            unsafe { dds_string_free(self.topic_name) };
        }
        if !self.type_name.is_null() {
            unsafe { dds_string_free(self.type_name) };
        }
        if !self.qos.is_null() {
            unsafe { dds_delete_qos(self.qos) };
        }
    }
}

impl DdsType for BuiltinTopicSample {
    fn type_name() -> &'static str {
        "DCPSTopic"
    }

    fn ops() -> Vec<u32> {
        Vec::new()
    }

    unsafe fn clone_out(ptr: *const Self) -> Self {
        let src = &*ptr;
        src.clone()
    }
}

impl std::fmt::Debug for BuiltinParticipantSample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinParticipantSample")
            .field("key", &self.key)
            .field("participant_name", &self.participant_name())
            .finish()
    }
}

impl std::fmt::Debug for BuiltinTopicSample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinTopicSample")
            .field("topic_name", &self.topic_name())
            .field("type_name", &self.type_name_value())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn builtin_topic_sample_clone_preserves_strings_and_qos() {
        let qos = Qos::builder()
            .reliable()
            .entity_name("builtin-topic")
            .build()
            .unwrap();
        let topic_name = CString::new("topic-a").unwrap();
        let type_name = CString::new("type-a").unwrap();

        let sample = BuiltinTopicSample {
            key: dds_builtintopic_topic_key_t { d: [1; 16] },
            topic_name: unsafe { dds_string_dup(topic_name.as_ptr()) },
            type_name: unsafe { dds_string_dup(type_name.as_ptr()) },
            qos: {
                let ptr = qos.as_ptr() as *mut dds_qos_t;
                std::mem::forget(qos);
                ptr
            },
        };

        let cloned = sample.clone();
        assert_eq!(cloned.topic_name(), "topic-a");
        assert_eq!(cloned.type_name_value(), "type-a");
        let cloned_qos = cloned.qos().unwrap().unwrap();
        assert_eq!(cloned_qos.entity_name().unwrap().unwrap(), "builtin-topic");
    }

    #[test]
    fn builtin_participant_sample_clone_preserves_qos() {
        let qos = Qos::builder()
            .entity_name("builtin-participant")
            .build()
            .unwrap();
        let sample = BuiltinParticipantSample {
            key: dds_guid_t { v: [9; 16] },
            qos: {
                let ptr = qos.as_ptr() as *mut dds_qos_t;
                std::mem::forget(qos);
                ptr
            },
        };

        let cloned = sample.clone();
        let cloned_qos = cloned.qos().unwrap().unwrap();
        assert_eq!(
            cloned_qos.entity_name().unwrap().unwrap(),
            "builtin-participant"
        );
        assert_eq!(cloned.key().v, [9; 16]);
    }
}

#[repr(C)]
pub struct BuiltinEndpointSample {
    key: dds_guid_t,
    participant_key: dds_guid_t,
    participant_instance_handle: dds_instance_handle_t,
    topic_name: *mut c_char,
    type_name: *mut c_char,
    qos: *mut dds_qos_t,
}

unsafe impl Send for BuiltinEndpointSample {}

impl BuiltinEndpointSample {
    fn as_native_ptr(&self) -> *mut dds_builtintopic_endpoint_t {
        self as *const Self as *mut dds_builtintopic_endpoint_t
    }

    pub fn key(&self) -> dds_guid_t {
        self.key
    }

    pub fn participant_key(&self) -> dds_guid_t {
        self.participant_key
    }

    pub fn participant_instance_handle(&self) -> dds_instance_handle_t {
        self.participant_instance_handle
    }

    pub fn topic_name(&self) -> String {
        if self.topic_name.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(self.topic_name) }
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn type_name_value(&self) -> String {
        if self.type_name.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(self.type_name) }
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Alias for [`type_name_value()`](Self::type_name_value) for ergonomic API
    /// consistency with the cyclonedds-python binding.
    pub fn type_name(&self) -> String {
        self.type_name_value()
    }

    pub fn qos(&self) -> DdsResult<Option<Qos>> {
        Qos::from_raw_clone(self.qos)
    }

    pub fn type_info(&self) -> DdsResult<TypeInfo> {
        let mut ptr = std::ptr::null();
        unsafe {
            check(dds_builtintopic_get_endpoint_type_info(
                self.as_native_ptr(),
                &mut ptr,
            ))?;
        }
        if ptr.is_null() {
            return Err(DdsError::Other(
                "CycloneDDS returned null endpoint typeinfo".into(),
            ));
        }
        Ok(TypeInfo::from_raw(ptr.cast_mut()))
    }

    pub fn create_topic_descriptor(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<TopicDescriptor> {
        self.type_info()?
            .create_topic_descriptor(participant, scope, timeout)
    }

    pub fn create_topic(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<UntypedTopic> {
        let descriptor = self.create_topic_descriptor(participant, scope, timeout)?;
        descriptor.create_topic(participant, &self.topic_name())
    }

    pub fn create_topic_with_qos(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
        qos: &Qos,
    ) -> DdsResult<UntypedTopic> {
        let descriptor = self.create_topic_descriptor(participant, scope, timeout)?;
        descriptor.create_topic_with_qos(participant, &self.topic_name(), qos)
    }

    pub fn find_topic(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<UntypedTopic>> {
        let type_info = self.type_info()?;
        let name = std::ffi::CString::new(self.topic_name())
            .map_err(|_| DdsError::BadParameter("topic name contains null".into()))?;
        let handle = unsafe {
            dds_find_topic(
                scope.as_raw(),
                participant,
                name.as_ptr(),
                type_info.as_ptr(),
                timeout,
            )
        };
        if handle == 0 {
            return Ok(None);
        }
        crate::error::check_entity(handle).map(|entity| Some(UntypedTopic::from_entity(entity)))
    }

    pub fn minimal_type_object(
        &self,
        participant: dds_entity_t,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<TypeObject>> {
        self.type_info()?.minimal_type_object(participant, timeout)
    }

    pub fn complete_type_object(
        &self,
        participant: dds_entity_t,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<TypeObject>> {
        self.type_info()?.complete_type_object(participant, timeout)
    }
}

impl Clone for BuiltinEndpointSample {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            participant_key: self.participant_key,
            participant_instance_handle: self.participant_instance_handle,
            topic_name: dup_cstr(self.topic_name),
            type_name: dup_cstr(self.type_name),
            qos: Qos::from_raw_clone(self.qos)
                .ok()
                .flatten()
                .map_or(std::ptr::null_mut(), |q| {
                    let ptr = q.as_ptr() as *mut dds_qos_t;
                    std::mem::forget(q);
                    ptr
                }),
        }
    }
}

impl Drop for BuiltinEndpointSample {
    fn drop(&mut self) {
        if !self.topic_name.is_null() {
            unsafe { dds_string_free(self.topic_name) };
        }
        if !self.type_name.is_null() {
            unsafe { dds_string_free(self.type_name) };
        }
        if !self.qos.is_null() {
            unsafe { dds_delete_qos(self.qos) };
        }
    }
}

impl DdsType for BuiltinEndpointSample {
    fn type_name() -> &'static str {
        "BuiltinEndpointSample"
    }

    fn ops() -> Vec<u32> {
        Vec::new()
    }

    unsafe fn clone_out(ptr: *const Self) -> Self {
        let src = &*ptr;
        src.clone()
    }
}

impl std::fmt::Debug for BuiltinEndpointSample {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BuiltinEndpointSample")
            .field("topic_name", &self.topic_name())
            .field("type_name", &self.type_name_value())
            .finish()
    }
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn builtin_endpoint_sample_clone_preserves_strings_and_qos() {
        let qos = Qos::builder()
            .reliable()
            .entity_name("builtin-endpoint")
            .build()
            .unwrap();
        let topic_name = CString::new("topic-endpoint").unwrap();
        let type_name = CString::new("type-endpoint").unwrap();

        let sample = BuiltinEndpointSample {
            key: dds_guid_t { v: [1; 16] },
            participant_key: dds_guid_t { v: [2; 16] },
            participant_instance_handle: 42,
            topic_name: unsafe { dds_string_dup(topic_name.as_ptr()) },
            type_name: unsafe { dds_string_dup(type_name.as_ptr()) },
            qos: {
                let ptr = qos.as_ptr() as *mut dds_qos_t;
                std::mem::forget(qos);
                ptr
            },
        };

        let cloned = sample.clone();
        assert_eq!(cloned.topic_name(), "topic-endpoint");
        assert_eq!(cloned.type_name_value(), "type-endpoint");
        assert_eq!(cloned.participant_instance_handle(), 42);
        let cloned_qos = cloned.qos().unwrap().unwrap();
        assert_eq!(
            cloned_qos.entity_name().unwrap().unwrap(),
            "builtin-endpoint"
        );
    }
}
