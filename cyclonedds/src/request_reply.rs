use crate::{
    entity::DdsEntity,
    error::{DdsError, DdsResult},
    DataReader, DataWriter, DomainParticipant, Publisher, Qos, Subscriber, DdsType,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Trait that types must implement to be used with Request-Reply pattern.
///
/// The correlation_id field is used to pair requests with replies.
///
/// # Example
/// ```no_run
/// #[derive(DdsTypeDerive, Clone)]
/// struct AddRequest {
///     correlation_id: u64,
///     a: i32,
///     b: i32,
/// }
///
/// impl RequestReply for AddRequest {
///     fn correlation_id(&self) -> u64 { self.correlation_id }
///     fn set_correlation_id(&mut self, id: u64) { self.correlation_id = id; }
/// }
/// ```
pub trait RequestReply: DdsType + Clone + Send + 'static {
    fn correlation_id(&self) -> u64;
    fn set_correlation_id(&mut self, id: u64);
}

/// A DDS Requester that sends requests and waits for matching replies.
///
/// Creates two topics internally:
/// - `{service_name}Request` — for publishing requests
/// - `{service_name}Reply` — for subscribing to replies
///
/// # Example
/// ```no_run
/// let participant = DomainParticipant::new(0).unwrap();
/// let requester = Requester::<AddRequest, AddReply>::new(
///     &participant, "CalcService", None, None,
/// ).unwrap();
///
/// let mut req = AddRequest { correlation_id: 0, a: 2, b: 3 };
/// let rep = requester.request(req, Duration::from_secs(1)).unwrap();
/// assert_eq!(rep.result, 5);
/// ```
pub struct Requester<TReq, TRep>
where
    TReq: RequestReply,
    TRep: RequestReply,
{
    _participant: DomainParticipant,
    _publisher: Publisher,
    _subscriber: Subscriber,
    writer: DataWriter<TReq>,
    reader: DataReader<TRep>,
    sequence: AtomicU64,
    _marker: std::marker::PhantomData<(TReq, TRep)>,
}

impl<TReq, TRep> Requester<TReq, TRep>
where
    TReq: RequestReply,
    TRep: RequestReply,
{
    pub fn new(
        participant: &DomainParticipant,
        service_name: &str,
        request_qos: Option<&Qos>,
        reply_qos: Option<&Qos>,
    ) -> DdsResult<Self> {
        let publisher = participant.create_publisher()?;
        let subscriber = participant.create_subscriber()?;

        let request_topic = if let Some(qos) = request_qos {
            participant.create_topic_with_qos::<TReq>(&format!("{}Request", service_name), qos)?
        } else {
            participant.create_topic::<TReq>(&format!("{}Request", service_name))?
        };
        let reply_topic = if let Some(qos) = reply_qos {
            participant.create_topic_with_qos::<TRep>(&format!("{}Reply", service_name), qos)?
        } else {
            participant.create_topic::<TRep>(&format!("{}Reply", service_name))?
        };

        let writer = if let Some(qos) = request_qos {
            publisher.create_writer_with_qos(&request_topic, qos)?
        } else {
            publisher.create_writer(&request_topic)?
        };

        let reader = if let Some(qos) = reply_qos {
            subscriber.create_reader_with_qos(&reply_topic, qos)?
        } else {
            subscriber.create_reader(&reply_topic)?
        };

        Ok(Requester {
            _participant: DomainParticipant::new(participant.entity() as u32)?,
            _publisher: publisher,
            _subscriber: subscriber,
            writer,
            reader,
            sequence: AtomicU64::new(1),
            _marker: std::marker::PhantomData,
        })
    }

    /// Send a request and block until the matching reply arrives or timeout.
    pub fn request(&self, mut data: TReq, timeout: Duration) -> DdsResult<TRep> {
        let seq = self.sequence.fetch_add(1, Ordering::SeqCst);
        data.set_correlation_id(seq);

        self.writer.write(&data)?;

        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            for sample in self.reader.take()? {
                if sample.correlation_id() == seq {
                    return Ok(sample);
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        Err(DdsError::Timeout)
    }
}

/// A DDS Replier that receives requests and sends replies.
///
/// Creates two topics internally:
/// - `{service_name}Request` — for subscribing to requests
/// - `{service_name}Reply` — for publishing replies
///
/// # Example
/// ```no_run
/// let participant = DomainParticipant::new(0).unwrap();
/// let replier = Replier::<AddRequest, AddReply>::new(
///     &participant, "CalcService", None, None,
/// ).unwrap();
///
/// loop {
///     if let Some(req) = replier.receive_request(Duration::from_secs(1)).unwrap() {
///         let reply = AddReply {
///             correlation_id: req.correlation_id(),
///             result: req.a + req.b,
///         };
///         replier.send_reply(reply).unwrap();
///     }
/// }
/// ```
pub struct Replier<TReq, TRep>
where
    TReq: RequestReply,
    TRep: RequestReply,
{
    _participant: DomainParticipant,
    _publisher: Publisher,
    _subscriber: Subscriber,
    writer: DataWriter<TRep>,
    reader: DataReader<TReq>,
    _marker: std::marker::PhantomData<(TReq, TRep)>,
}

impl<TReq, TRep> Replier<TReq, TRep>
where
    TReq: RequestReply,
    TRep: RequestReply,
{
    pub fn new(
        participant: &DomainParticipant,
        service_name: &str,
        request_qos: Option<&Qos>,
        reply_qos: Option<&Qos>,
    ) -> DdsResult<Self> {
        let publisher = participant.create_publisher()?;
        let subscriber = participant.create_subscriber()?;

        let request_topic = if let Some(qos) = request_qos {
            participant.create_topic_with_qos::<TReq>(&format!("{}Request", service_name), qos)?
        } else {
            participant.create_topic::<TReq>(&format!("{}Request", service_name))?
        };
        let reply_topic = if let Some(qos) = reply_qos {
            participant.create_topic_with_qos::<TRep>(&format!("{}Reply", service_name), qos)?
        } else {
            participant.create_topic::<TRep>(&format!("{}Reply", service_name))?
        };

        let reader = if let Some(qos) = request_qos {
            subscriber.create_reader_with_qos(&request_topic, qos)?
        } else {
            subscriber.create_reader(&request_topic)?
        };

        let writer = if let Some(qos) = reply_qos {
            publisher.create_writer_with_qos(&reply_topic, qos)?
        } else {
            publisher.create_writer(&reply_topic)?
        };

        Ok(Replier {
            _participant: DomainParticipant::new(participant.entity() as u32)?,
            _publisher: publisher,
            _subscriber: subscriber,
            writer,
            reader,
            _marker: std::marker::PhantomData,
        })
    }

    /// Block until a request arrives or timeout.
    pub fn receive_request(&self, timeout: Duration) -> DdsResult<Option<TReq>> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            for sample in self.reader.take()? {
                return Ok(Some(sample));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(None)
    }

    /// Send a reply with the matching correlation_id.
    pub fn send_reply(&self, data: TRep) -> DdsResult<()> {
        self.writer.write(&data)
    }
}
