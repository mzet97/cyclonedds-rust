use cyclonedds::{DdsEntity, DdsError, DomainParticipant};

#[test]
fn error_is_transient_detects_recoverable_errors() {
    assert!(DdsError::Timeout.is_transient());
    assert!(DdsError::OutOfResources.is_transient());
    assert!(DdsError::OutOfMemory.is_transient());
    assert!(DdsError::ReturnCode(-99).is_transient());
    assert!(!DdsError::BadParameter("x".into()).is_transient());
    assert!(!DdsError::AlreadyDeleted.is_transient());
    assert!(!DdsError::PreconditionNotMet("x".into()).is_transient());
}

#[test]
fn participant_new_with_retry_succeeds_first_attempt() {
    let participant = DomainParticipant::new_with_retry(0, 3, 10);
    assert!(participant.is_ok());
}

#[test]
fn participant_status_changes_returns_zero_for_new_participant() {
    let participant = DomainParticipant::new(0).unwrap();
    let changes = participant.status_changes().unwrap();
    assert_eq!(changes, 0);
}
