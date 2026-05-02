use crate::{
    builtin::{
        BuiltinEndpointSample, BuiltinParticipantSample, BuiltinTopicSample,
        BUILTIN_TOPIC_DCPSPARTICIPANT, BUILTIN_TOPIC_DCPSPUBLICATION,
        BUILTIN_TOPIC_DCPSSUBSCRIPTION, BUILTIN_TOPIC_DCPSTOPIC,
    },
    entity::DdsEntity,
    listener::Listener,
    xtypes::{FindScope, SertypeHandle, TopicDescriptor, TypeInfo},
    DataReader, DdsError, DdsResult, DynamicType, DynamicTypeBuilder, History, Publisher, Qos,
    Reliability, Subscriber, Topic, UntypedTopic,
};
use cyclonedds_rust_sys::*;

/// RAII guard for a temporary writer entity.
struct WriterGuard {
    entity: dds_entity_t,
}

impl Drop for WriterGuard {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}

/// RAII guard for a temporary reader entity.
struct ReaderGuard {
    entity: dds_entity_t,
}

impl Drop for ReaderGuard {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}

/// A DDS DomainParticipant is the entry-point for DDS communication.
///
/// It represents the local membership of the application in a DDS domain
/// identified by a domain ID. All other DDS entities (topics, publishers,
/// subscribers, readers, writers) are created from a participant.
pub struct DomainParticipant {
    entity: dds_entity_t,
}

impl DomainParticipant {
    /// Create a new participant in the given domain with default QoS.
    ///
    /// # Example
    /// ```no_run
    /// use cyclonedds::DomainParticipant;
    /// let participant = DomainParticipant::new(0).unwrap();
    /// ```
    #[cfg_attr(feature = "tracing", tracing::instrument)]
    pub fn new(domain_id: u32) -> DdsResult<Self> {
        Self::with_qos_and_listener(domain_id, None, None)
    }

    /// Create a new participant with retry on transient errors.
    ///
    /// Retries up to `max_retries` times with exponential backoff
    /// starting at `base_delay_ms`.
    pub fn new_with_retry(
        domain_id: u32,
        max_retries: u32,
        base_delay_ms: u64,
    ) -> DdsResult<Self> {
        let mut delay = std::time::Duration::from_millis(base_delay_ms);
        for attempt in 0..=max_retries {
            match Self::new(domain_id) {
                Ok(p) => return Ok(p),
                Err(e) if attempt < max_retries && e.is_transient() => {
                    std::thread::sleep(delay);
                    delay *= 2;
                }
                Err(e) => return Err(e),
            }
        }
        Err(DdsError::Timeout)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(qos)))]
    pub fn with_qos(domain_id: u32, qos: Option<&Qos>) -> DdsResult<Self> {
        Self::with_qos_and_listener(domain_id, qos, None)
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(listener)))]
    pub fn with_listener(domain_id: u32, listener: &Listener) -> DdsResult<Self> {
        Self::with_qos_and_listener(domain_id, None, Some(listener))
    }

    #[cfg_attr(feature = "tracing", tracing::instrument(skip(qos, listener)))]
    pub fn with_qos_and_listener(
        domain_id: u32,
        qos: Option<&Qos>,
        listener: Option<&Listener>,
    ) -> DdsResult<Self> {
        unsafe {
            let handle = dds_create_participant(
                domain_id,
                qos.map_or(std::ptr::null(), |q| q.as_ptr()),
                listener.map_or(std::ptr::null(), |l| l.as_ptr() as *const _),
            );
            crate::error::check_entity(handle)?;
            Ok(DomainParticipant { entity: handle })
        }
    }

    pub fn create_topic<T: crate::DdsType>(&self, name: &str) -> DdsResult<Topic<T>> {
        Topic::new(self.entity, name)
    }

    pub fn create_topic_with_qos<T: crate::DdsType>(
        &self,
        name: &str,
        qos: &Qos,
    ) -> DdsResult<Topic<T>> {
        Topic::with_qos(self.entity, name, Some(qos))
    }

    pub fn create_topic_from_descriptor(
        &self,
        name: &str,
        descriptor: &TopicDescriptor,
    ) -> DdsResult<UntypedTopic> {
        UntypedTopic::from_descriptor(self.entity, name, descriptor)
    }

    pub fn create_topic_from_descriptor_with_qos(
        &self,
        name: &str,
        descriptor: &TopicDescriptor,
        qos: &Qos,
    ) -> DdsResult<UntypedTopic> {
        UntypedTopic::from_descriptor_with_qos(self.entity, name, descriptor, Some(qos))
    }

    pub fn create_topic_descriptor(
        &self,
        type_info: &TypeInfo,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<TopicDescriptor> {
        type_info.create_topic_descriptor(self.entity, scope, timeout)
    }

    pub fn create_topic_from_type_info(
        &self,
        name: &str,
        type_info: &TypeInfo,
        scope: FindScope,
        timeout: dds_duration_t,
    ) -> DdsResult<UntypedTopic> {
        type_info.create_topic(self.entity, scope, timeout, name)
    }

    pub fn create_topic_from_type_info_with_qos(
        &self,
        name: &str,
        type_info: &TypeInfo,
        scope: FindScope,
        timeout: dds_duration_t,
        qos: &Qos,
    ) -> DdsResult<UntypedTopic> {
        type_info.create_topic_with_qos(self.entity, scope, timeout, name, qos)
    }

    pub fn create_topic_from_sertype(
        &self,
        name: &str,
        sertype: &SertypeHandle,
    ) -> DdsResult<UntypedTopic> {
        sertype.create_topic(self.entity, name, None)
    }

    pub fn create_topic_from_sertype_with_qos(
        &self,
        name: &str,
        sertype: &SertypeHandle,
        qos: &Qos,
    ) -> DdsResult<UntypedTopic> {
        sertype.create_topic(self.entity, name, Some(qos))
    }

    pub fn find_topic(
        &self,
        scope: FindScope,
        name: &str,
        type_info: Option<&TypeInfo>,
        timeout: dds_duration_t,
    ) -> DdsResult<Option<UntypedTopic>> {
        let name = std::ffi::CString::new(name)
            .map_err(|_| crate::DdsError::BadParameter("topic name contains null".into()))?;
        let handle = unsafe {
            dds_find_topic(
                scope.as_raw(),
                self.entity,
                name.as_ptr(),
                type_info.map_or(std::ptr::null(), TypeInfo::as_ptr),
                timeout,
            )
        };
        if handle == 0 {
            return Ok(None);
        }
        crate::error::check_entity(handle).map(|entity| Some(UntypedTopic::from_entity(entity)))
    }

    pub fn create_dynamic_type(&self, builder: DynamicTypeBuilder) -> DdsResult<DynamicType> {
        DynamicType::create(self.entity, builder)
    }

    // ── Dynamic data I/O convenience methods (Part 4.3) ──

    /// Publish a `DynamicData` sample on a dynamically-created topic.
    ///
    /// This creates a topic from the `DynamicType`'s registered type info,
    /// creates a temporary publisher and writer, writes the sample via native
    /// buffer, then cleans up all temporary entities.
    ///
    /// For repeated publishing, prefer creating the topic and writer explicitly
    /// and using [`dynamic_data_to_cdr`] for better performance.
    ///
    /// [`dynamic_data_to_cdr`]: crate::type_discovery::dynamic_data_to_cdr
    pub fn dynamic_publish(
        &self,
        topic_name: &str,
        dynamic_type: &mut DynamicType,
        data: &crate::DynamicData,
    ) -> DdsResult<()> {
        let descriptor = dynamic_type.register_topic_descriptor(self, FindScope::Global, 0)?;

        let topic = descriptor.create_topic(self.entity, topic_name)?;

        let publisher = Publisher::new(self.entity)?;
        let qos = Qos::builder()
            .reliability(Reliability::Reliable, 0)
            .build()?;
        let writer = unsafe {
            let handle = dds_create_writer(
                publisher.entity(),
                topic.entity(),
                qos.as_ptr(),
                std::ptr::null_mut(),
            );
            crate::error::check_entity(handle)?;
            WriterGuard { entity: handle }
        };

        // Build a native sample buffer and write the DynamicValue into it
        let size = descriptor.size() as usize;
        let align = std::cmp::max(descriptor.align() as usize, 1);
        let layout = std::alloc::Layout::from_size_align(size, align)
            .map_err(|_| DdsError::BadParameter("invalid type layout for dynamic data".into()))?;

        unsafe {
            let buf = std::alloc::alloc_zeroed(layout);
            if buf.is_null() {
                return Err(DdsError::OutOfMemory);
            }

            // Write the DynamicValue into the native buffer
            crate::type_discovery::write_value_to_native(data.value(), buf, descriptor.ops(), 0);

            // Write via native pointer
            let ret = dds_write(writer.entity, buf as *const std::ffi::c_void);

            // Free any dynamically-allocated members the write may have created
            dds_stream_free_sample(
                buf as *mut std::ffi::c_void,
                &dds_cdrstream_default_allocator,
                descriptor.ops().as_ptr(),
            );
            std::alloc::dealloc(buf, layout);

            crate::error::check(ret)?;
        }

        Ok(())
    }

    /// Subscribe to a dynamically-typed topic and read available samples.
    ///
    /// Creates a topic, subscriber, and reader from the `DynamicType`'s
    /// registered type info, reads all currently available samples as native
    /// buffers, then extracts field values into `DynamicData`.
    ///
    /// For repeated reading, prefer creating the topic and reader explicitly
    /// and using [`cdr_to_dynamic_data`] for better performance.
    ///
    /// [`cdr_to_dynamic_data`]: crate::type_discovery::cdr_to_dynamic_data
    pub fn dynamic_subscribe(
        &self,
        topic_name: &str,
        dynamic_type: &mut DynamicType,
        max_samples: usize,
    ) -> DdsResult<Vec<crate::DynamicData>> {
        let schema = dynamic_type.schema().clone();
        let descriptor = dynamic_type.register_topic_descriptor(self, FindScope::Global, 0)?;

        let topic = descriptor.create_topic(self.entity, topic_name)?;

        let subscriber = Subscriber::new(self.entity)?;
        let qos = Qos::builder()
            .reliability(Reliability::Reliable, 0)
            .history(History::KeepAll)
            .build()?;
        let reader = unsafe {
            let handle = dds_create_reader(
                subscriber.entity(),
                topic.entity(),
                qos.as_ptr(),
                std::ptr::null_mut(),
            );
            crate::error::check_entity(handle)?;
            ReaderGuard { entity: handle }
        };

        // Read samples as native buffers using dds_take
        let max = max_samples.max(256).min(1024);
        let mut samples: Vec<*mut std::ffi::c_void> = vec![std::ptr::null_mut(); max];
        let mut infos: Vec<dds_sample_info_t> = vec![unsafe { std::mem::zeroed() }; max];

        let n = unsafe {
            dds_take(
                reader.entity,
                samples.as_mut_ptr(),
                infos.as_mut_ptr() as *mut dds_sample_info_t,
                max,
                max as u32,
            )
        };

        if n < 0 {
            return Err(DdsError::from(n));
        }
        let n = n as usize;

        let mut results = Vec::with_capacity(n);
        for i in 0..n {
            if !infos[i].valid_data || samples[i].is_null() {
                continue;
            }

            // Extract DynamicValue from the native buffer
            let value = crate::type_discovery::read_value_from_native_public(
                samples[i] as *mut u8,
                &schema,
                descriptor.ops(),
                0,
            );

            match crate::DynamicData::from_value(&schema, value) {
                Ok(data) => results.push(data),
                Err(_) => continue,
            }
        }

        // Return the loan
        let _ = unsafe { dds_return_loan(reader.entity, samples.as_mut_ptr(), n as i32) };

        Ok(results)
    }

    pub fn create_builtin_participant_reader(
        &self,
    ) -> DdsResult<DataReader<BuiltinParticipantSample>> {
        DataReader::new(self.entity, BUILTIN_TOPIC_DCPSPARTICIPANT)
    }

    pub fn create_builtin_participant_reader_with_qos(
        &self,
        qos: &Qos,
    ) -> DdsResult<DataReader<BuiltinParticipantSample>> {
        DataReader::with_qos(self.entity, BUILTIN_TOPIC_DCPSPARTICIPANT, Some(qos))
    }

    pub fn create_builtin_topic_reader(&self) -> DdsResult<DataReader<BuiltinTopicSample>> {
        DataReader::new(self.entity, BUILTIN_TOPIC_DCPSTOPIC)
    }

    pub fn create_builtin_topic_reader_with_qos(
        &self,
        qos: &Qos,
    ) -> DdsResult<DataReader<BuiltinTopicSample>> {
        DataReader::with_qos(self.entity, BUILTIN_TOPIC_DCPSTOPIC, Some(qos))
    }

    pub fn create_builtin_publication_reader(
        &self,
    ) -> DdsResult<DataReader<BuiltinEndpointSample>> {
        DataReader::new(self.entity, BUILTIN_TOPIC_DCPSPUBLICATION)
    }

    pub fn create_builtin_publication_reader_with_qos(
        &self,
        qos: &Qos,
    ) -> DdsResult<DataReader<BuiltinEndpointSample>> {
        DataReader::with_qos(self.entity, BUILTIN_TOPIC_DCPSPUBLICATION, Some(qos))
    }

    pub fn create_builtin_subscription_reader(
        &self,
    ) -> DdsResult<DataReader<BuiltinEndpointSample>> {
        DataReader::new(self.entity, BUILTIN_TOPIC_DCPSSUBSCRIPTION)
    }

    pub fn create_builtin_subscription_reader_with_qos(
        &self,
        qos: &Qos,
    ) -> DdsResult<DataReader<BuiltinEndpointSample>> {
        DataReader::with_qos(self.entity, BUILTIN_TOPIC_DCPSSUBSCRIPTION, Some(qos))
    }

    pub fn create_publisher(&self) -> DdsResult<Publisher> {
        Publisher::new(self.entity)
    }

    pub fn create_subscriber(&self) -> DdsResult<Subscriber> {
        Subscriber::new(self.entity)
    }

    // -----------------------------------------------------------------------
    // Domain deaf/mute control
    // -----------------------------------------------------------------------

    /// Set the participant to *deaf* mode: it will ignore incoming network
    /// traffic for the given `duration`.
    ///
    /// Pass `deaf = false` to restore normal operation immediately
    /// (`duration` is ignored in that case).
    pub fn set_deaf(&self, deaf: bool, duration: dds_duration_t) -> DdsResult<()> {
        unsafe { crate::error::check(dds_domain_set_deafmute(self.entity, deaf, false, duration)) }
    }

    /// Set the participant to *mute* mode: it will not send any outgoing
    /// network traffic for the given `duration`.
    ///
    /// Pass `mute = false` to restore normal operation immediately
    /// (`duration` is ignored in that case).
    pub fn set_mute(&self, mute: bool, duration: dds_duration_t) -> DdsResult<()> {
        unsafe { crate::error::check(dds_domain_set_deafmute(self.entity, false, mute, duration)) }
    }

    /// Set both *deaf* and *mute* on the participant simultaneously.
    pub fn set_deaf_mute(&self, deaf: bool, mute: bool, duration: dds_duration_t) -> DdsResult<()> {
        unsafe { crate::error::check(dds_domain_set_deafmute(self.entity, deaf, mute, duration)) }
    }

    // -----------------------------------------------------------------------
    // Admin / Discovery API
    // -----------------------------------------------------------------------

    /// Return all discovered participants in the domain.
    pub fn discovered_participants(&self) -> DdsResult<Vec<BuiltinParticipantSample>> {
        let reader = self.create_builtin_participant_reader()?;
        reader.take()
    }

    /// Return all discovered publications (DataWriters) in the domain.
    pub fn discovered_publications(&self) -> DdsResult<Vec<BuiltinEndpointSample>> {
        let reader = self.create_builtin_publication_reader()?;
        reader.take()
    }

    /// Return all discovered subscriptions (DataReaders) in the domain.
    pub fn discovered_subscriptions(&self) -> DdsResult<Vec<BuiltinEndpointSample>> {
        let reader = self.create_builtin_subscription_reader()?;
        reader.take()
    }

    /// Return all discovered topics in the domain.
    pub fn discovered_topics(&self) -> DdsResult<Vec<BuiltinTopicSample>> {
        let reader = self.create_builtin_topic_reader()?;
        reader.take()
    }
}

impl DdsEntity for DomainParticipant {
    fn entity(&self) -> dds_entity_t {
        self.entity
    }
}

impl Drop for DomainParticipant {
    fn drop(&mut self) {
        unsafe {
            dds_delete(self.entity);
        }
    }
}
