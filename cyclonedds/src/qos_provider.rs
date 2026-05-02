use crate::{error::check, DdsError, DdsResult, Qos};
use cyclonedds_rust_sys::{
    dds_create_qos_provider, dds_create_qos_provider_scope, dds_delete_qos_provider,
    dds_qos_kind_DDS_PARTICIPANT_QOS, dds_qos_kind_DDS_PUBLISHER_QOS, dds_qos_kind_DDS_READER_QOS,
    dds_qos_kind_DDS_SUBSCRIBER_QOS, dds_qos_kind_DDS_TOPIC_QOS, dds_qos_kind_DDS_WRITER_QOS,
    dds_qos_kind_t, dds_qos_provider_get_qos, dds_qos_provider_t,
};
use std::ffi::CString;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QosKind {
    Participant,
    Publisher,
    Subscriber,
    Topic,
    Reader,
    Writer,
}

impl QosKind {
    fn as_raw(self) -> dds_qos_kind_t {
        match self {
            QosKind::Participant => dds_qos_kind_DDS_PARTICIPANT_QOS as dds_qos_kind_t,
            QosKind::Publisher => dds_qos_kind_DDS_PUBLISHER_QOS as dds_qos_kind_t,
            QosKind::Subscriber => dds_qos_kind_DDS_SUBSCRIBER_QOS as dds_qos_kind_t,
            QosKind::Topic => dds_qos_kind_DDS_TOPIC_QOS as dds_qos_kind_t,
            QosKind::Reader => dds_qos_kind_DDS_READER_QOS as dds_qos_kind_t,
            QosKind::Writer => dds_qos_kind_DDS_WRITER_QOS as dds_qos_kind_t,
        }
    }
}

pub struct QosProvider {
    ptr: *mut dds_qos_provider_t,
}

impl QosProvider {
    pub fn new(definition: &str) -> DdsResult<Self> {
        let definition = to_cstring(definition, "qos definition")?;
        let mut ptr = std::ptr::null_mut();
        unsafe {
            check(dds_create_qos_provider(definition.as_ptr(), &mut ptr))?;
        }
        if ptr.is_null() {
            return Err(DdsError::OutOfResources);
        }
        Ok(Self { ptr })
    }

    /// Create a provider from an inline XML QoS definition.
    ///
    /// Alias for [`Self::new`].
    pub fn from_xml(definition: &str) -> DdsResult<Self> {
        Self::new(definition)
    }

    /// Create a provider from an inline XML QoS definition and select a
    /// named profile (scope).
    ///
    /// The `scope` corresponds to the `name` attribute of a `<qos_profile>`
    /// element in the XML.
    pub fn from_xml_with_profile(definition: &str, profile: &str) -> DdsResult<Self> {
        Self::with_scope(definition, profile)
    }

    pub fn with_scope(definition: &str, scope: &str) -> DdsResult<Self> {
        let definition = to_cstring(definition, "qos definition")?;
        let scope = to_cstring(scope, "qos scope")?;
        let mut ptr = std::ptr::null_mut();
        unsafe {
            check(dds_create_qos_provider_scope(
                definition.as_ptr(),
                &mut ptr,
                scope.as_ptr(),
            ))?;
        }
        if ptr.is_null() {
            return Err(DdsError::OutOfResources);
        }
        Ok(Self { ptr })
    }

    pub fn from_file(path: impl AsRef<Path>) -> DdsResult<Self> {
        let path = path.as_ref();
        let definition = path.to_str().ok_or_else(|| {
            DdsError::BadParameter(format!("path is not valid UTF-8: {}", path.display()))
        })?;
        Self::new(definition)
    }

    pub fn from_file_with_scope(path: impl AsRef<Path>, scope: &str) -> DdsResult<Self> {
        let path = path.as_ref();
        let definition = path.to_str().ok_or_else(|| {
            DdsError::BadParameter(format!("path is not valid UTF-8: {}", path.display()))
        })?;
        Self::with_scope(definition, scope)
    }

    /// Create a provider from an XML file and select a named profile.
    ///
    /// The `profile` corresponds to the `name` attribute of a `<qos_profile>`
    /// element in the XML file.
    pub fn from_file_with_profile(path: impl AsRef<Path>, profile: &str) -> DdsResult<Self> {
        Self::from_file_with_scope(path, profile)
    }

    pub fn get_qos(&self, kind: QosKind, key: &str) -> DdsResult<Qos> {
        let key = to_cstring(key, "qos key")?;
        let mut qos = std::ptr::null();
        unsafe {
            check(dds_qos_provider_get_qos(
                self.ptr,
                kind.as_raw(),
                key.as_ptr(),
                &mut qos,
            ))?;
        }
        Qos::from_raw_clone(qos)?.ok_or_else(|| {
            DdsError::Other(format!("CycloneDDS returned a null QoS for key `{key:?}`"))
        })
    }

    /// Convenience: get participant QoS for a named profile.
    pub fn get_participant_qos(&self, profile: &str) -> DdsResult<Qos> {
        self.get_qos(QosKind::Participant, profile)
    }

    /// Convenience: get publisher QoS for a named profile.
    pub fn get_publisher_qos(&self, profile: &str) -> DdsResult<Qos> {
        self.get_qos(QosKind::Publisher, profile)
    }

    /// Convenience: get subscriber QoS for a named profile.
    pub fn get_subscriber_qos(&self, profile: &str) -> DdsResult<Qos> {
        self.get_qos(QosKind::Subscriber, profile)
    }

    /// Convenience: get topic QoS for a named profile.
    pub fn get_topic_qos(&self, profile: &str) -> DdsResult<Qos> {
        self.get_qos(QosKind::Topic, profile)
    }

    /// Convenience: get data reader QoS for a named profile.
    pub fn get_reader_qos(&self, profile: &str) -> DdsResult<Qos> {
        self.get_qos(QosKind::Reader, profile)
    }

    /// Convenience: get data writer QoS for a named profile.
    pub fn get_writer_qos(&self, profile: &str) -> DdsResult<Qos> {
        self.get_qos(QosKind::Writer, profile)
    }
}

impl Drop for QosProvider {
    fn drop(&mut self) {
        unsafe {
            dds_delete_qos_provider(self.ptr);
        }
    }
}

fn to_cstring(value: &str, label: &str) -> DdsResult<CString> {
    CString::new(value)
        .map_err(|_| DdsError::BadParameter(format!("{label} contains an interior null byte")))
}
