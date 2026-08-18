//! `TopicDescriptor` must survive being cloned.
//!
//! It carried a `Clone` that copied the raw `*mut dds_topic_descriptor_t` with
//! no reference count, and a `Drop` that called `dds_delete_topic_descriptor`
//! on it. Two clones therefore owned the same pointer: the second drop is a
//! double free, and any access after the first drop is a use-after-free.
//!
//! Nothing in this repository cloned one, which is why it never fired — but
//! `Clone` is public API, so any caller doing the obvious thing hit it.
//!
//! Run under AddressSanitizer for a deterministic verdict:
//! ```bash
//! RUSTFLAGS="-Zsanitizer=address" cargo +nightly test -p cyclonedds-test-suite \
//!     --target x86_64-unknown-linux-gnu --test descriptor_clone
//! ```

use cyclonedds::*;
use cyclonedds_test_suite::unique_topic;

fn build_descriptor(participant: &DomainParticipant) -> TopicDescriptor {
    let mut dynamic_type = participant
        .create_dynamic_type(DynamicTypeBuilder::structure(unique_topic(
            "desc_clone_type",
        )))
        .unwrap();
    dynamic_type
        .add_member(DynamicMemberBuilder::primitive("id", DynamicPrimitiveKind::UInt32).id(10))
        .unwrap();
    dynamic_type.set_member_key(10, true).unwrap();
    dynamic_type
        .register_topic_descriptor(participant, FindScope::LocalDomain, 0)
        .unwrap()
}

#[test]
fn cloned_descriptor_does_not_double_free() {
    let participant = DomainParticipant::new(0).unwrap();
    let original = build_descriptor(&participant);

    let copy = original.clone();
    assert_eq!(copy.key_count(), original.key_count());
    assert!(copy.op_count() > 0);

    // Dropping one clone must leave the other fully usable: the descriptor is
    // only released once the last owner goes away.
    drop(original);
    assert!(
        copy.op_count() > 0,
        "descriptor freed while a clone was live"
    );
    assert_eq!(copy.key_count(), 1);

    // And dropping the survivor must not free a second time.
    drop(copy);
}

#[test]
fn many_clones_release_exactly_once() {
    let participant = DomainParticipant::new(0).unwrap();
    let original = build_descriptor(&participant);

    let clones: Vec<TopicDescriptor> = (0..8).map(|_| original.clone()).collect();
    for c in &clones {
        assert!(c.op_count() > 0);
    }
    drop(original);
    for c in &clones {
        assert!(c.op_count() > 0, "descriptor freed while clones were live");
    }
    drop(clones);
}

/// A clone must still be usable to create a topic after the original is gone.
#[test]
fn cloned_descriptor_still_creates_a_topic() {
    let participant = DomainParticipant::new(0).unwrap();
    let original = build_descriptor(&participant);
    let copy = original.clone();
    drop(original);

    let topic_name = unique_topic("desc_clone_topic");
    let topic = participant
        .create_topic_from_descriptor(&topic_name, &copy)
        .expect("cloned descriptor should still create a topic");
    assert_eq!(topic.get_name().unwrap(), topic_name);
}
