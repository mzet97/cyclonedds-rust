use cyclonedds::{DdsEntity, DdsTypeDerive, DomainParticipant, Subscriber};
use futures_util::StreamExt;

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct AsyncSample {
    id: i32,
}

#[tokio::test]
async fn read_aiter_timeout_returns_empty_on_no_data() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic = participant.create_topic::<AsyncSample>("AsyncTimeoutTest").unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let reader: cyclonedds::DataReader<AsyncSample> =
        cyclonedds::DataReader::new(subscriber.entity(), topic.entity()).unwrap();

    let mut stream = Box::pin(reader.read_aiter_timeout(50_000_000));
    let batch = stream.next().await.unwrap().unwrap();
    assert!(batch.is_empty());
}

#[tokio::test]
async fn take_aiter_timeout_returns_empty_on_no_data() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic = participant.create_topic::<AsyncSample>("AsyncTimeoutTest2").unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let reader: cyclonedds::DataReader<AsyncSample> =
        cyclonedds::DataReader::new(subscriber.entity(), topic.entity()).unwrap();

    let mut stream = Box::pin(reader.take_aiter_timeout(50_000_000));
    let batch = stream.next().await.unwrap().unwrap();
    assert!(batch.is_empty());
}

#[tokio::test]
async fn stream_can_be_dropped_without_panic() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic = participant.create_topic::<AsyncSample>("AsyncDropTest").unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let reader: cyclonedds::DataReader<AsyncSample> =
        cyclonedds::DataReader::new(subscriber.entity(), topic.entity()).unwrap();

    {
        let mut stream = Box::pin(reader.read_aiter_timeout(50_000_000));
        let _ = stream.next().await;
        // stream dropped here — should not panic or leak
    }
}
