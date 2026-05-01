use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cyclonedds::{CdrDeserializer, CdrEncoding, CdrSerializer};
use cyclonedds_derive::DdsTypeDerive;

#[derive(DdsTypeDerive, Clone)]
struct Point {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(DdsTypeDerive, Clone)]
struct Pose {
    position: Point,
    orientation: Point,
}

#[derive(DdsTypeDerive, Clone)]
struct Message {
    id: i32,
    seq: u64,
    payload: String,
}

fn bench_cdr_serialize(c: &mut Criterion) {
    let point = Point { x: 1.0, y: 2.0, z: 3.0 };
    let pose = Pose {
        position: point.clone(),
        orientation: point.clone(),
    };
    let msg = Message {
        id: 42,
        seq: 123456789,
        payload: "Hello, DDS!".to_string(),
    };

    c.bench_function("cdr_serialize_point", |b| {
        b.iter(|| {
            let _ = CdrSerializer::serialize(black_box(&point), CdrEncoding::Xcdr1).unwrap();
        })
    });

    c.bench_function("cdr_serialize_pose", |b| {
        b.iter(|| {
            let _ = CdrSerializer::serialize(black_box(&pose), CdrEncoding::Xcdr1).unwrap();
        })
    });

    c.bench_function("cdr_serialize_message", |b| {
        b.iter(|| {
            let _ = CdrSerializer::serialize(black_box(&msg), CdrEncoding::Xcdr1).unwrap();
        })
    });
}

fn bench_cdr_deserialize(c: &mut Criterion) {
    let point = Point { x: 1.0, y: 2.0, z: 3.0 };
    let pose = Pose {
        position: point.clone(),
        orientation: point.clone(),
    };
    let msg = Message {
        id: 42,
        seq: 123456789,
        payload: "Hello, DDS!".to_string(),
    };

    let point_bytes = CdrSerializer::serialize(&point, CdrEncoding::Xcdr1).unwrap();
    let pose_bytes = CdrSerializer::serialize(&pose, CdrEncoding::Xcdr1).unwrap();
    let msg_bytes = CdrSerializer::serialize(&msg, CdrEncoding::Xcdr1).unwrap();

    c.bench_function("cdr_deserialize_point", |b| {
        b.iter(|| {
            let _: Point = CdrDeserializer::deserialize(black_box(&point_bytes), CdrEncoding::Xcdr1).unwrap();
        })
    });

    c.bench_function("cdr_deserialize_pose", |b| {
        b.iter(|| {
            let _: Pose = CdrDeserializer::deserialize(black_box(&pose_bytes), CdrEncoding::Xcdr1).unwrap();
        })
    });

    c.bench_function("cdr_deserialize_message", |b| {
        b.iter(|| {
            let _: Message = CdrDeserializer::deserialize(black_box(&msg_bytes), CdrEncoding::Xcdr1).unwrap();
        })
    });
}

fn bench_cdr_roundtrip(c: &mut Criterion) {
    let point = Point { x: 1.0, y: 2.0, z: 3.0 };

    c.bench_function("cdr_roundtrip_point", |b| {
        b.iter(|| {
            let bytes = CdrSerializer::serialize(black_box(&point), CdrEncoding::Xcdr1).unwrap();
            let _: Point = CdrDeserializer::deserialize(&bytes, CdrEncoding::Xcdr1).unwrap();
        })
    });
}

fn bench_pod_memcpy(c: &mut Criterion) {
    // Benchmark manual memcpy for a POD type as a comparison baseline.
    let point = Point { x: 1.0, y: 2.0, z: 3.0 };

    c.bench_function("pod_memcpy_point", |b| {
        b.iter(|| {
            let src = black_box(&point);
            let mut dst = Point { x: 0.0, y: 0.0, z: 0.0 };
            unsafe {
                std::ptr::copy_nonoverlapping(
                    src as *const Point as *const u8,
                    &mut dst as *mut Point as *mut u8,
                    std::mem::size_of::<Point>(),
                );
            }
            black_box(dst);
        })
    });
}

criterion_group!(
    benches,
    bench_cdr_serialize,
    bench_cdr_deserialize,
    bench_cdr_roundtrip,
    bench_pod_memcpy,
);
criterion_main!(benches);
