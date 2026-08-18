use cyclonedds::{CdrEncoding, CdrSerializer, DdsTypeDerive};

#[repr(C)]
#[derive(Debug, Clone, DdsTypeDerive)]
struct KeyedSample {
    #[key]
    id: i32,
    payload: i64,
}

#[test]
fn key_serialization_links_against_shared_cyclonedds() {
    let sample = KeyedSample { id: 7, payload: 9 };
    let encoded = CdrSerializer::<KeyedSample>::serialize_key(&sample, CdrEncoding::Xcdr1)
        .expect("key serialization through exported CycloneDDS APIs must succeed");

    let mut buffer = [0_u8; 64];
    let written = CdrSerializer::<KeyedSample>::serialize_key_to_buffer(
        &sample,
        &mut buffer,
        CdrEncoding::Xcdr1,
    )
    .expect("buffered key serialization through exported CycloneDDS APIs must succeed");

    assert_eq!(&buffer[..written], encoded);
}
