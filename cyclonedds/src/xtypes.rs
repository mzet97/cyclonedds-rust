use crate::{
    error::check, qos::Qos, DdsError, DdsResult, TopicKeyDescriptor, UntypedTopic,
    OP_ADR, OP_DLC, OP_FLAG_EXT, OP_FLAG_KEY, OP_FLAG_MU, OP_FLAG_OPT,
    OP_JEQ4, OP_KOF, OP_RTS, OP_MID,
    TYPE_1BY, TYPE_2BY, TYPE_4BY, TYPE_8BY,
    TYPE_ARR, TYPE_BSQ, TYPE_BST, TYPE_ENU, TYPE_EXT, TYPE_SEQ, TYPE_STR, TYPE_UNI,
    DDS_OP_TYPE_MASK_CONST, DDS_OP_SUBTYPE_MASK_CONST, DDS_OP_MASK_CONST,
    SUBTYPE_BST,
};
use cyclonedds_sys::*;
use std::ffi::CStr;

pub struct TypeInfo {
    ptr: *mut dds_typeinfo_t,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeIncludeDeps {
    Ignore,
    Include,
}

impl TypeIncludeDeps {
    fn as_raw(self) -> ddsi_type_include_deps_t {
        match self {
            Self::Ignore => DDSI_TYPE_IGNORE_DEPS,
            Self::Include => DDSI_TYPE_INCLUDE_DEPS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeIdKind {
    Minimal,
    Complete,
    PlainCollectionMinimal,
    PlainCollectionComplete,
    FullyDescriptive,
    Invalid,
}

pub struct TypeIdRef<'a> {
    ptr: *const dds_typeid_t,
    _marker: std::marker::PhantomData<&'a TypeInfo>,
}

pub struct OwnedTypeId {
    ptr: *mut dds_typeid_t,
}

impl<'a> TypeIdRef<'a> {
    pub fn as_ptr(&self) -> *const dds_typeid_t {
        self.ptr
    }

    pub fn equals(&self, other: &TypeIdRef<'_>) -> bool {
        unsafe { ddsi_typeid_compare(self.ptr, other.ptr) == 0 }
    }

    pub fn is_none(&self) -> bool {
        unsafe { ddsi_typeid_is_none(self.ptr) }
    }

    pub fn is_hash(&self) -> bool {
        unsafe { ddsi_typeid_is_hash(self.ptr) }
    }

    pub fn is_minimal(&self) -> bool {
        unsafe { ddsi_typeid_is_minimal(self.ptr) }
    }

    pub fn is_complete(&self) -> bool {
        unsafe { ddsi_typeid_is_complete(self.ptr) }
    }

    pub fn is_fully_descriptive(&self) -> bool {
        unsafe { ddsi_typeid_is_fully_descriptive(self.ptr) }
    }

    pub fn kind(&self) -> TypeIdKind {
        match unsafe { ddsi_typeid_kind(self.ptr) } {
            DDSI_TYPEID_KIND_MINIMAL => TypeIdKind::Minimal,
            DDSI_TYPEID_KIND_COMPLETE => TypeIdKind::Complete,
            DDSI_TYPEID_KIND_PLAIN_COLLECTION_MINIMAL => TypeIdKind::PlainCollectionMinimal,
            DDSI_TYPEID_KIND_PLAIN_COLLECTION_COMPLETE => TypeIdKind::PlainCollectionComplete,
            DDSI_TYPEID_KIND_FULLY_DESCRIPTIVE => TypeIdKind::FullyDescriptive,
            _ => TypeIdKind::Invalid,
        }
    }

    pub fn resolve_type_object(
        &self,
        entity: dds_entity_t,
        timeout: dds_duration_t,
    ) -> DdsResult<TypeObject> {
        TypeObject::from_entity_type_id(entity, self.ptr, timeout)
    }

    pub fn equivalence_hash(&self) -> [u8; 14] {
        let mut hash = [0u8; 14];
        unsafe { ddsi_typeid_get_equivalence_hash(self.ptr, &mut hash) };
        hash
    }

    pub fn display_string(&self) -> String {
        let mut buf = ddsi_typeid_str { str_: [0; 50] };
        let ptr = unsafe { ddsi_make_typeid_str(&mut buf, self.ptr) };
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn to_owned_type_id(&self) -> DdsResult<OwnedTypeId> {
        let ptr = unsafe { ddsi_typeid_dup(self.ptr) };
        if ptr.is_null() {
            Err(DdsError::OutOfResources)
        } else {
            Ok(OwnedTypeId { ptr })
        }
    }
}

impl OwnedTypeId {
    pub fn as_ptr(&self) -> *const dds_typeid_t {
        self.ptr.cast_const()
    }

    pub fn equals_ref(&self, other: &TypeIdRef<'_>) -> bool {
        unsafe { ddsi_typeid_compare(self.as_ptr(), other.as_ptr()) == 0 }
    }

    pub fn equals(&self, other: &OwnedTypeId) -> bool {
        unsafe { ddsi_typeid_compare(self.as_ptr(), other.as_ptr()) == 0 }
    }

    pub fn is_none(&self) -> bool {
        unsafe { ddsi_typeid_is_none(self.as_ptr()) }
    }

    pub fn is_hash(&self) -> bool {
        unsafe { ddsi_typeid_is_hash(self.as_ptr()) }
    }

    pub fn is_minimal(&self) -> bool {
        unsafe { ddsi_typeid_is_minimal(self.as_ptr()) }
    }

    pub fn is_complete(&self) -> bool {
        unsafe { ddsi_typeid_is_complete(self.as_ptr()) }
    }

    pub fn is_fully_descriptive(&self) -> bool {
        unsafe { ddsi_typeid_is_fully_descriptive(self.as_ptr()) }
    }

    pub fn kind(&self) -> TypeIdKind {
        match unsafe { ddsi_typeid_kind(self.as_ptr()) } {
            DDSI_TYPEID_KIND_MINIMAL => TypeIdKind::Minimal,
            DDSI_TYPEID_KIND_COMPLETE => TypeIdKind::Complete,
            DDSI_TYPEID_KIND_PLAIN_COLLECTION_MINIMAL => TypeIdKind::PlainCollectionMinimal,
            DDSI_TYPEID_KIND_PLAIN_COLLECTION_COMPLETE => TypeIdKind::PlainCollectionComplete,
            DDSI_TYPEID_KIND_FULLY_DESCRIPTIVE => TypeIdKind::FullyDescriptive,
            _ => TypeIdKind::Invalid,
        }
    }

    pub fn equivalence_hash(&self) -> [u8; 14] {
        let mut hash = [0u8; 14];
        unsafe { ddsi_typeid_get_equivalence_hash(self.as_ptr(), &mut hash) };
        hash
    }

    pub fn display_string(&self) -> String {
        let mut buf = ddsi_typeid_str { str_: [0; 50] };
        let ptr = unsafe { ddsi_make_typeid_str(&mut buf, self.as_ptr()) };
        if ptr.is_null() {
            String::new()
        } else {
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned()
        }
    }

    pub fn resolve_type_object(
        &self,
        entity: dds_entity_t,
        timeout: dds_duration_t,
    ) -> DdsResult<TypeObject> {
        TypeObject::from_entity_type_id(entity, self.as_ptr(), timeout)
    }
}

impl Drop for OwnedTypeId {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                ddsi_typeid_fini(self.ptr);
                dds_free(self.ptr.cast());
            }
        }
    }
}

pub struct SertypeHandle {
    ptr: *const ddsi_sertype,
}

pub struct OwnedSertype {
    ptr: *mut ddsi_sertype,
}

impl SertypeHandle {
    pub(crate) fn from_raw(ptr: *const ddsi_sertype) -> DdsResult<Self> {
        if ptr.is_null() {
            Err(DdsError::Other("CycloneDDS returned null sertype".into()))
        } else {
            Ok(Self { ptr })
        }
    }

    pub fn as_ptr(&self) -> *const ddsi_sertype {
        self.ptr
    }

    pub fn hash(&self) -> u32 {
        unsafe { ddsi_sertype_hash(self.ptr) }
    }

    pub fn equals(&self, other: &SertypeHandle) -> bool {
        unsafe { ddsi_sertype_equal(self.ptr, other.ptr) }
    }

    pub fn clone_ref(&self) -> DdsResult<OwnedSertype> {
        let ptr = unsafe { ddsi_sertype_ref(self.ptr) };
        if ptr.is_null() {
            Err(DdsError::OutOfResources)
        } else {
            Ok(OwnedSertype { ptr })
        }
    }

    pub fn create_topic(
        &self,
        participant: dds_entity_t,
        name: &str,
        qos: Option<&Qos>,
    ) -> DdsResult<UntypedTopic> {
        self.clone_ref()?.create_topic(participant, name, qos)
    }
}

impl OwnedSertype {
    pub fn as_ptr(&self) -> *const ddsi_sertype {
        self.ptr.cast_const()
    }

    pub fn hash(&self) -> u32 {
        unsafe { ddsi_sertype_hash(self.ptr) }
    }

    pub fn equals(&self, other: &SertypeHandle) -> bool {
        unsafe { ddsi_sertype_equal(self.ptr, other.as_ptr()) }
    }

    pub fn create_topic(
        mut self,
        participant: dds_entity_t,
        name: &str,
        qos: Option<&Qos>,
    ) -> DdsResult<UntypedTopic> {
        let name = std::ffi::CString::new(name)
            .map_err(|_| DdsError::BadParameter("topic name contains null".into()))?;
        let mut sertype = self.ptr;
        let handle = unsafe {
            dds_create_topic_sertype(
                participant,
                name.as_ptr(),
                &mut sertype,
                qos.map_or(std::ptr::null(), |q| q.as_ptr()),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        crate::error::check_entity(handle)?;
        self.ptr = std::ptr::null_mut();
        Ok(UntypedTopic::from_entity(handle))
    }
}

impl Drop for OwnedSertype {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ddsi_sertype_unref(self.ptr) };
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindScope {
    Global,
    LocalDomain,
    Participant,
}

impl FindScope {
    pub(crate) fn as_raw(self) -> dds_find_scope_t {
        match self {
            Self::Global => dds_find_scope_DDS_FIND_SCOPE_GLOBAL,
            Self::LocalDomain => dds_find_scope_DDS_FIND_SCOPE_LOCAL_DOMAIN,
            Self::Participant => dds_find_scope_DDS_FIND_SCOPE_PARTICIPANT,
        }
    }
}

pub struct TopicDescriptor {
    ptr: *mut dds_topic_descriptor_t,
}

impl std::fmt::Debug for TopicDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TopicDescriptor").finish()
    }
}

impl Clone for TopicDescriptor {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl TopicDescriptor {
    pub(crate) fn from_raw(ptr: *mut dds_topic_descriptor_t) -> Self {
        Self { ptr }
    }

    pub fn as_ptr(&self) -> *const dds_topic_descriptor_t {
        self.ptr.cast_const()
    }

    pub fn create_topic(&self, participant: dds_entity_t, name: &str) -> DdsResult<UntypedTopic> {
        UntypedTopic::from_descriptor(participant, name, self)
    }

    pub fn create_topic_with_qos(
        &self,
        participant: dds_entity_t,
        name: &str,
        qos: &Qos,
    ) -> DdsResult<UntypedTopic> {
        UntypedTopic::from_descriptor_with_qos(participant, name, self, Some(qos))
    }

    pub fn size(&self) -> u32 {
        unsafe { (*self.ptr).m_size }
    }

    pub fn align(&self) -> u32 {
        unsafe { (*self.ptr).m_align }
    }

    pub fn key_count(&self) -> u32 {
        unsafe { (*self.ptr).m_nkeys }
    }

    pub fn flagset(&self) -> u32 {
        unsafe { (*self.ptr).m_flagset }
    }

    pub fn op_count(&self) -> u32 {
        unsafe { (*self.ptr).m_nops }
    }

    pub fn ops(&self) -> &[u32] {
        unsafe {
            if (*self.ptr).m_ops.is_null() || (*self.ptr).m_nops == 0 {
                &[]
            } else {
                std::slice::from_raw_parts((*self.ptr).m_ops, (*self.ptr).m_nops as usize)
            }
        }
    }

    pub fn key_descriptors(&self) -> Vec<TopicKeyDescriptor> {
        unsafe {
            if (*self.ptr).m_keys.is_null() || (*self.ptr).m_nkeys == 0 {
                return Vec::new();
            }
            std::slice::from_raw_parts((*self.ptr).m_keys, (*self.ptr).m_nkeys as usize)
                .iter()
                .map(|key| TopicKeyDescriptor {
                    name: if key.m_name.is_null() {
                        String::new()
                    } else {
                        CStr::from_ptr(key.m_name).to_string_lossy().into_owned()
                    },
                    offset: key.m_offset,
                    index: key.m_idx,
                })
                .collect()
        }
    }

    pub fn metadata_xml(&self) -> String {
        unsafe {
            let ptr = (*self.ptr).m_meta;
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    pub fn type_information_bytes(&self) -> &[u8] {
        unsafe {
            let meta = &(*self.ptr).type_information;
            if meta.data.is_null() || meta.sz == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(meta.data, meta.sz as usize)
            }
        }
    }

    pub fn type_mapping_bytes(&self) -> &[u8] {
        unsafe {
            let meta = &(*self.ptr).type_mapping;
            if meta.data.is_null() || meta.sz == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(meta.data, meta.sz as usize)
            }
        }
    }

    pub fn restrict_data_representation(&self) -> u32 {
        unsafe { (*self.ptr).restrict_data_representation }
    }

    pub fn type_name(&self) -> String {
        unsafe {
            let ptr = (*self.ptr).m_typename;
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }
}

impl Drop for TopicDescriptor {
    fn drop(&mut self) {
        unsafe {
            let _ = dds_delete_topic_descriptor(self.ptr);
        }
    }
}

impl TypeInfo {
    pub(crate) fn from_raw(ptr: *mut dds_typeinfo_t) -> Self {
        Self { ptr }
    }

    pub(crate) fn from_entity(entity: dds_entity_t) -> DdsResult<Self> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            check(dds_get_typeinfo(entity, &mut ptr))?;
        }
        if ptr.is_null() {
            return Err(DdsError::Other("CycloneDDS returned null typeinfo".into()));
        }
        Ok(Self { ptr })
    }

    pub fn as_ptr(&self) -> *const dds_typeinfo_t {
        self.ptr.cast_const()
    }

    pub fn is_present(&self) -> bool {
        self.minimal_type_id().is_some() || self.complete_type_id().is_some()
    }

    pub fn is_valid(&self) -> bool {
        self.is_present()
    }

    pub fn minimal_type_id(&self) -> Option<TypeIdRef<'_>> {
        let ptr = unsafe { ddsi_typeinfo_minimal_typeid(self.as_ptr()) };
        (!ptr.is_null()).then_some(TypeIdRef {
            ptr,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn complete_type_id(&self) -> Option<TypeIdRef<'_>> {
        let ptr = unsafe { ddsi_typeinfo_complete_typeid(self.as_ptr()) };
        (!ptr.is_null()).then_some(TypeIdRef {
            ptr,
            _marker: std::marker::PhantomData,
        })
    }

    pub fn matches(&self, other: &TypeInfo) -> bool {
        match (
            self.minimal_type_id(),
            other.minimal_type_id(),
            self.complete_type_id(),
            other.complete_type_id(),
        ) {
            (Some(a_min), Some(b_min), Some(a_compl), Some(b_compl)) => {
                a_min.equals(&b_min) && a_compl.equals(&b_compl)
            }
            (Some(a_min), Some(b_min), None, None) => a_min.equals(&b_min),
            (None, None, Some(a_compl), Some(b_compl)) => a_compl.equals(&b_compl),
            _ => false,
        }
    }

    pub fn equals(&self, other: &TypeInfo, deps: TypeIncludeDeps) -> bool {
        unsafe { ddsi_typeinfo_equal(self.as_ptr(), other.as_ptr(), deps.as_raw()) }
    }

    pub fn type_id(&self, kind: TypeIdKind) -> Option<TypeIdRef<'_>> {
        match kind {
            TypeIdKind::Minimal => self.minimal_type_id(),
            TypeIdKind::Complete => self.complete_type_id(),
            _ => None,
        }
    }

    pub fn type_id_owned(&self, kind: TypeIdKind) -> DdsResult<Option<OwnedTypeId>> {
        match self.type_id(kind) {
            Some(type_id) => type_id.to_owned_type_id().map(Some),
            None => Ok(None),
        }
    }

    pub fn duplicate(&self) -> DdsResult<TypeInfo> {
        let ptr = unsafe { ddsi_typeinfo_dup(self.as_ptr()) };
        if ptr.is_null() {
            Err(DdsError::OutOfResources)
        } else {
            Ok(TypeInfo::from_raw(ptr))
        }
    }

    pub fn minimal_type_object(
        &self,
        entity: dds_entity_t,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<TypeObject>> {
        match self.minimal_type_id() {
            Some(type_id) => type_id.resolve_type_object(entity, timeout).map(Some),
            None => Ok(None),
        }
    }

    pub fn complete_type_object(
        &self,
        entity: dds_entity_t,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<TypeObject>> {
        match self.complete_type_id() {
            Some(type_id) => type_id.resolve_type_object(entity, timeout).map(Some),
            None => Ok(None),
        }
    }

    pub fn create_topic_descriptor(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<TopicDescriptor> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            check(dds_create_topic_descriptor(
                scope.as_raw(),
                participant,
                self.as_ptr(),
                timeout,
                &mut ptr,
            ))?;
        }
        if ptr.is_null() {
            return Err(DdsError::Other(
                "CycloneDDS returned null topic descriptor".into(),
            ));
        }
        Ok(TopicDescriptor::from_raw(ptr))
    }

    pub fn create_topic(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
        name: &str,
    ) -> DdsResult<UntypedTopic> {
        let descriptor = self.create_topic_descriptor(participant, scope, timeout)?;
        descriptor.create_topic(participant, name)
    }

    pub fn create_topic_with_qos(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
        name: &str,
        qos: &Qos,
    ) -> DdsResult<UntypedTopic> {
        let descriptor = self.create_topic_descriptor(participant, scope, timeout)?;
        descriptor.create_topic_with_qos(participant, name, qos)
    }

    pub fn find_topic(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
        name: &str,
    ) -> DdsResult<Option<UntypedTopic>> {
        let name = std::ffi::CString::new(name)
            .map_err(|_| DdsError::BadParameter("topic name contains null".into()))?;
        let handle = unsafe {
            dds_find_topic(
                scope.as_raw(),
                participant,
                name.as_ptr(),
                self.as_ptr(),
                timeout,
            )
        };
        if handle == 0 {
            return Ok(None);
        }
        crate::error::check_entity(handle).map(|entity| Some(UntypedTopic::from_entity(entity)))
    }
}

impl Drop for TypeInfo {
    fn drop(&mut self) {
        unsafe {
            let _ = dds_free_typeinfo(self.ptr);
        }
    }
}

pub struct TypeObject {
    ptr: *mut dds_typeobj_t,
}

impl TypeObject {
    pub(crate) fn from_entity_type_id(
        entity: dds_entity_t,
        type_id: *const dds_typeid_t,
        timeout: dds_duration_t,
    ) -> DdsResult<Self> {
        let mut ptr = std::ptr::null_mut();
        unsafe {
            check(dds_get_typeobj(entity, type_id, timeout, &mut ptr))?;
        }
        if ptr.is_null() {
            return Err(DdsError::Other(
                "CycloneDDS returned null type object".into(),
            ));
        }
        Ok(Self { ptr })
    }

    pub fn as_ptr(&self) -> *const dds_typeobj_t {
        self.ptr.cast_const()
    }
}

impl Drop for TypeObject {
    fn drop(&mut self) {
        unsafe {
            let _ = dds_free_typeobj(self.ptr);
        }
    }
}

pub struct MatchedEndpoint {
    ptr: *mut dds_builtintopic_endpoint_t,
}

impl MatchedEndpoint {
    pub(crate) fn from_subscription(
        writer: dds_entity_t,
        handle: dds_instance_handle_t,
    ) -> DdsResult<Self> {
        let ptr = unsafe { dds_get_matched_subscription_data(writer, handle) };
        if ptr.is_null() {
            return Err(DdsError::Other(
                "failed to fetch matched subscription endpoint".into(),
            ));
        }
        Ok(Self { ptr })
    }

    pub(crate) fn from_publication(
        reader: dds_entity_t,
        handle: dds_instance_handle_t,
    ) -> DdsResult<Self> {
        let ptr = unsafe { dds_get_matched_publication_data(reader, handle) };
        if ptr.is_null() {
            return Err(DdsError::Other(
                "failed to fetch matched publication endpoint".into(),
            ));
        }
        Ok(Self { ptr })
    }

    pub fn key(&self) -> dds_guid_t {
        unsafe { (*self.ptr).key }
    }

    pub fn participant_key(&self) -> dds_guid_t {
        unsafe { (*self.ptr).participant_key }
    }

    pub fn participant_instance_handle(&self) -> dds_instance_handle_t {
        unsafe { (*self.ptr).participant_instance_handle }
    }

    pub fn topic_name(&self) -> String {
        unsafe {
            let ptr = (*self.ptr).topic_name;
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    pub fn type_name(&self) -> String {
        unsafe {
            let ptr = (*self.ptr).type_name;
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    pub fn qos(&self) -> DdsResult<Option<Qos>> {
        unsafe { Qos::from_raw_clone((*self.ptr).qos) }
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
        self.type_info()?
            .find_topic(participant, scope, timeout, &self.topic_name())
    }

    pub fn type_info(&self) -> DdsResult<TypeInfo> {
        let mut ptr = std::ptr::null();
        unsafe {
            check(dds_builtintopic_get_endpoint_type_info(self.ptr, &mut ptr))?;
        }
        if ptr.is_null() {
            return Err(DdsError::Other(
                "CycloneDDS returned null endpoint typeinfo".into(),
            ));
        }
        Ok(TypeInfo {
            ptr: ptr.cast_mut(),
        })
    }

    pub fn minimal_type_object(
        &self,
        entity: dds_entity_t,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<TypeObject>> {
        self.type_info()?.minimal_type_object(entity, timeout)
    }

    pub fn complete_type_object(
        &self,
        entity: dds_entity_t,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<TypeObject>> {
        self.type_info()?.complete_type_object(entity, timeout)
    }
}

impl Drop for MatchedEndpoint {
    fn drop(&mut self) {
        unsafe {
            dds_builtintopic_free_endpoint(self.ptr);
        }
    }
}

// ---------------------------------------------------------------------------
// High-level XTypes type-descriptor wrappers (Phase 7.3)
// ---------------------------------------------------------------------------

/// Kind of a DDS type (matches XTypes TypeKind).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Struct,
    Union,
    Enum,
    Bitmask,
    Alias,
    Sequence,
    Array,
    String,
    Map,
    Primitive,
}

/// Extensibility of a DDS type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeExtensibility {
    Final,
    Appendable,
    Mutable,
}

/// Description of a single type member (struct field, union case, enum literal).
#[derive(Debug, Clone)]
pub struct MemberDescriptor {
    pub name: String,
    pub id: u32,
    pub type_kind: TypeKind,
    pub is_key: bool,
    pub is_optional: bool,
    pub is_external: bool,
    pub is_must_understand: bool,
}

/// High-level navigable type descriptor parsed from a [`TopicDescriptor`]'s ops.
#[derive(Debug, Clone)]
pub struct TypeDescriptor {
    pub name: String,
    pub kind: TypeKind,
    pub extensibility: TypeExtensibility,
    pub members: Vec<MemberDescriptor>,
    pub key_count: u32,
    pub has_nested_types: bool,
}

/// Classify the primary type from an ADR opcode's type bits.
fn classify_primary_type(primary: u32) -> TypeKind {
    match primary {
        TYPE_1BY | TYPE_2BY | TYPE_4BY | TYPE_8BY => TypeKind::Primitive,
        TYPE_STR => TypeKind::String,
        TYPE_BST => TypeKind::String,
        TYPE_SEQ => TypeKind::Sequence,
        TYPE_BSQ => TypeKind::Sequence,
        TYPE_ARR => TypeKind::Array,
        TYPE_ENU => TypeKind::Enum,
        TYPE_EXT => TypeKind::Struct,
        TYPE_UNI => TypeKind::Union,
        _ => TypeKind::Primitive,
    }
}

/// Determine the step size (number of u32 words consumed) for an ADR opcode
/// starting at position `i` in the ops array.
fn adr_step(ops: &[u32], i: usize) -> usize {
    if i >= ops.len() {
        return ops.len() - i;
    }
    let op = ops[i];
    let primary = op & DDS_OP_TYPE_MASK_CONST;
    let subtype = op & DDS_OP_SUBTYPE_MASK_CONST;
    match primary {
        TYPE_BST => 3,
        TYPE_SEQ => {
            if subtype == SUBTYPE_BST {
                3
            } else {
                2
            }
        }
        TYPE_BSQ => {
            if subtype == SUBTYPE_BST {
                4
            } else {
                3
            }
        }
        _ => 2,
    }
}

impl TopicDescriptor {
    /// Parse the ops array into a high-level [`TypeDescriptor`].
    ///
    /// This walks the topic descriptor's streaming ops to extract member
    /// information, key count, extensibility, and nested-type presence.
    /// Member names are sourced from key descriptors when available, or
    /// synthesised from the member index otherwise.
    pub fn parse_type(&self) -> DdsResult<TypeDescriptor> {
        let ops = self.ops();
        let flagset = self.flagset();
        let nkeys = self.key_count();
        let key_descs = self.key_descriptors();
        let type_name = self.type_name();
        let metadata = self.metadata_xml();

        // Determine extensibility from the flagset.
        // OP_FLAG_EXT bit set in flagset means mutable extensibility.
        // A non-zero flagset without OP_FLAG_EXT typically means appendable.
        // Zero flagset means final.
        let extensibility = if (flagset & OP_FLAG_EXT) != 0 {
            TypeExtensibility::Mutable
        } else if flagset != 0 {
            TypeExtensibility::Appendable
        } else {
            TypeExtensibility::Final
        };

        // Detect overall type kind from ops content.
        let mut has_union = false;
        let mut has_enum = false;
        let mut has_nested = false;
        let mut member_index: u32 = 0;
        let mut members: Vec<MemberDescriptor> = Vec::new();

        let mut i: usize = 0;
        while i < ops.len() {
            let op = ops[i];
            let opcode = op & DDS_OP_MASK_CONST;

            match opcode {
                OP_ADR => {
                    let primary = op & DDS_OP_TYPE_MASK_CONST;
                    let kind = classify_primary_type(primary);
                    let is_key = (op & OP_FLAG_KEY) != 0;
                    let is_optional = (op & OP_FLAG_OPT) != 0;
                    let is_mu = (op & OP_FLAG_MU) != 0;

                    if matches!(kind, TypeKind::Struct) {
                        has_nested = true;
                    }

                    // Try to find name from key descriptors.
                    let name = key_descs
                        .iter()
                        .find(|kd| kd.index == member_index)
                        .map(|kd| kd.name.clone())
                        .unwrap_or_else(|| format!("field_{}", member_index));

                    members.push(MemberDescriptor {
                        name,
                        id: member_index,
                        type_kind: kind,
                        is_key,
                        is_optional,
                        is_external: false,
                        is_must_understand: is_mu,
                    });

                    member_index += 1;
                    i += adr_step(ops, i);
                }
                OP_DLC => {
                    // Delimiter close -- skip
                    i += 1;
                }
                OP_RTS => {
                    // Return from subtopic -- end of top-level type
                    break;
                }
                OP_JEQ4 => {
                    // Jump-equal for unions: 4 words wide
                    i += 4;
                }
                OP_KOF => {
                    // Key offset format: skip the count + indices
                    if i + 1 < ops.len() {
                        let count = ops[i] & 0x00FFFFFF;
                        i += 1 + count as usize;
                    } else {
                        i += 1;
                    }
                }
                OP_MID => {
                    // Member ID: skip (2 words: MID + actual id value)
                    i += 2;
                }
                _ => {
                    // Unknown opcode -- skip one word
                    i += 1;
                }
            }
        }

        // Check the metadata XML for union/enum hints if available
        if !metadata.is_empty() {
            if metadata.contains("<union") || metadata.contains("union ") {
                has_union = true;
            }
            if metadata.contains("<enum") || metadata.contains("enum ") {
                has_enum = true;
            }
        }

        // Also inspect ops directly for TYPE_UNI / TYPE_ENU opcodes
        for &op in ops.iter() {
            let primary = op & DDS_OP_TYPE_MASK_CONST;
            if primary == TYPE_UNI {
                has_union = true;
            }
            if primary == TYPE_ENU {
                has_enum = true;
            }
        }

        let kind = if has_union {
            TypeKind::Union
        } else if has_enum && members.is_empty() {
            TypeKind::Enum
        } else {
            TypeKind::Struct
        };

        Ok(TypeDescriptor {
            name: type_name,
            kind,
            extensibility,
            members,
            key_count: nkeys,
            has_nested_types: has_nested,
        })
    }
}

impl TypeInfo {
    /// Convenience method: create a topic descriptor and parse it into a
    /// high-level [`TypeDescriptor`] in a single call.
    pub fn parse_type_descriptor(
        &self,
        participant: dds_entity_t,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<TypeDescriptor> {
        let desc = self.create_topic_descriptor(participant, scope, timeout)?;
        desc.parse_type()
    }
}
