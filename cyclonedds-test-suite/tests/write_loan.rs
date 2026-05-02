use cyclonedds::*;
use std::time::Duration;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct LoanTestMsg {
    id: i32,
    value: i64,
}

impl LoanTestMsg {
    fn new(id: i32, value: i64) -> Self {
        Self { id, value }
    }
}

impl DdsType for LoanTestMsg {
    fn type_name() -> &'static str {
        "LoanTestMsg"
    }
    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        ops.extend(adr(TYPE_4BY | OP_FLAG_SGN, 0));
        ops.extend(adr(TYPE_8BY | OP_FLAG_SGN, 8));
        ops
    }
}

/// Test zero-copy write loan lifecycle: request, populate, write, receive.
#[test]
fn zero_copy_loan_write_and_read() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic = participant
        .create_topic::<LoanTestMsg>("loan_test")
        .unwrap();
    let publisher = participant.create_publisher().unwrap();
    let subscriber = participant.create_subscriber().unwrap();
    let writer = publisher.create_writer(&topic).unwrap();
    let reader = subscriber.create_reader(&topic).unwrap();

    // Wait for matching
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if !writer.matched_subscriptions().unwrap().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // Request loan, populate, and write
    let mut loan = writer.request_loan().unwrap();
    {
        let sample = loan.get_mut();
        sample.id = 42;
        sample.value = 12345;
    }
    WriteLoan::write(loan).unwrap();

    // Read back
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        if let Some(sample) = reader.take().unwrap().into_iter().next() {
            assert_eq!(sample.id, 42);
            assert_eq!(sample.value, 12345);
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    panic!("did not receive loaned sample");
}

/// Test that dropping a loan without writing returns it safely.
#[test]
fn zero_copy_loan_drop_without_write() {
    let participant = DomainParticipant::new(0).unwrap();
    let topic = participant
        .create_topic::<LoanTestMsg>("loan_test_drop")
        .unwrap();
    let publisher = participant.create_publisher().unwrap();
    let writer = publisher.create_writer(&topic).unwrap();

    // Request and drop without writing — should not panic or leak
    {
        let mut loan = writer.request_loan().unwrap();
        let sample = loan.get_mut();
        sample.id = 99;
        sample.value = 999;
        // loan dropped here
    }

    // Should be able to request another loan after drop
    let mut loan2 = writer.request_loan().unwrap();
    let sample2 = loan2.get_mut();
    assert_eq!(sample2.id, 0); // zero-initialized
    assert_eq!(sample2.value, 0);
}
