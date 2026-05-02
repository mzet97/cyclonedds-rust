use cyclonedds::{DdsEntity, DdsTypeDerive, DomainParticipant, Publisher, Subscriber};

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct LoanSample {
    id: i32,
}

#[test]
fn read_loan_returns_empty_when_no_data() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic = participant.create_topic::<LoanSample>("LoanTest").unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let reader: cyclonedds::DataReader<LoanSample> =
        cyclonedds::DataReader::new(subscriber.entity(), topic.entity()).unwrap();

    let loan = reader.read_loan().unwrap();
    assert_eq!(loan.len(), 0);
}

#[test]
fn take_loan_returns_empty_when_no_data() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic = participant.create_topic::<LoanSample>("LoanTest2").unwrap();
    let subscriber = Subscriber::new(participant.entity()).unwrap();
    let reader: cyclonedds::DataReader<LoanSample> =
        cyclonedds::DataReader::new(subscriber.entity(), topic.entity()).unwrap();

    let loan = reader.take_loan().unwrap();
    assert_eq!(loan.len(), 0);
}
